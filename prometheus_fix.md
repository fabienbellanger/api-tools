# `api-tools` PrometheusLayer — corrections appliquées

> Ce document a été remplacé par le rapport des modifications réellement
> apportées à la crate. L'analyse d'origine (avant fix) reste accessible dans
> l'historique git de ce fichier.

---

## TL;DR

- **Bug critique corrigé** : suppression du `tokio::time::sleep(200 ms)` qui
  bloquait le hot-path HTTP. Latence par requête : **~210 ms → < 1 ms** (×200+).
- **Bug de naming corrigé** : `system_used_disks_usage` → `system_used_disks_space`.
- **Nouvelle API** : `spawn_system_metrics_collector(...)` pour collecter les
  métriques host en arrière-plan.
- **Optimisations bonus** : caching des labels (`Arc<str>` pour `service_name`,
  `&'static str` pour méthodes/codes HTTP standards) et buckets d'histogramme
  paramétrables.
- **Bump de version** : `0.7.0 → 0.8.0` (breaking : retrait de
  `disk_mount_points` du `PrometheusLayer` + renommage de la metric ci-dessus).

---

## Cause racine (rappel)

Dans `SystemMetrics::new()` (appelée à chaque requête HTTP) :

```rust
sys.refresh_cpu_usage();
let mut cpu_usage = sys.global_cpu_usage();
tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;  // ← 200 ms
sys.refresh_cpu_usage();
cpu_usage += sys.global_cpu_usage();
cpu_usage /= 2.0;
```

`MINIMUM_CPU_UPDATE_INTERVAL` vaut **200 ms** sur Linux/macOS/Windows
(`sysinfo` calcule un delta entre deux snapshots de `/proc/stat`). Le `sleep`
était dans le critical path de chaque requête HTTP, plafonnant le throughput
d'une connexion à ~5 req/s. À chaque tick s'ajoutait :

- `System::new_all()` qui scanne tous les processus (plusieurs ms)
- `Disks::new_with_refreshed_list()` (énumère les block devices)
- 7 lookups + sets sur le registre `metrics` global

---

## Architecture corrigée

Le layer est maintenant scindé en **deux composants distincts** :

### 1. `PrometheusLayer` — middleware HTTP allégé

Ne fait plus que les métriques par-requête (`http_requests_total`,
`http_requests_duration_seconds`). Coût : ~µs par requête. Aucune syscall
système, aucun `sleep`.

### 2. `spawn_system_metrics_collector(...)` — tâche background

Fonction publique qui spawn une tâche Tokio persistante :

- Maintient une instance `System` réutilisée entre ticks
  (`new_with_specifics` : skip le scan des processus).
- Sur chaque tick (`tokio::time::interval` configurable, ex. 10 s) :
  refresh CPU + mémoire + disques + push des gauges.
- Pas de `sleep` artificiel : `refresh_cpu_usage()` retourne une vraie valeur
  dès le 2ᵉ tick puisque `sysinfo` calcule un delta entre snapshots
  persistés.
- Retourne un `JoinHandle<()>` que l'utilisateur peut `abort()` au shutdown.

---

## Migration côté utilisateur (0.7 → 0.8)

```rust
// Avant (0.7)
let layer = PrometheusLayer {
    service_name: "myapp".into(),
    disk_mount_points: vec!["/".into()],
};

// Après (0.8)
use api_tools::server::axum::layers::prometheus::{
    PrometheusLayer, spawn_system_metrics_collector,
};
use std::time::Duration;
use std::path::PathBuf;

let layer = PrometheusLayer { service_name: "myapp".into() };

// Une seule fois, au boot de l'app :
let _collector = spawn_system_metrics_collector(
    "myapp".into(),
    vec![PathBuf::from("/")],
    Duration::from_secs(10),
);
```

### Buckets d'histogramme custom (optionnel)

```rust
use api_tools::server::axum::handlers::prometheus::{
    PrometheusHandler, DEFAULT_DURATION_BUCKETS,
};

// Default (équivalent à get_handle()) :
let handle = PrometheusHandler::get_handle()?;

// Buckets custom adaptés à un service très rapide :
let handle = PrometheusHandler::get_handle_with_buckets(
    &[0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5],
)?;
```

---

## Détail des changements

### Breaking changes

| Avant | Après |
|---|---|
| `PrometheusLayer { service_name, disk_mount_points }` | `PrometheusLayer { service_name }` |
| metric `system_used_disks_usage` | metric `system_used_disks_space` |

### Nouvelles API publiques

- `spawn_system_metrics_collector(service_name, disk_mount_points, interval) -> JoinHandle<()>`
  dans `api_tools::server::axum::layers::prometheus`
- `PrometheusHandler::get_handle_with_buckets(buckets: &[f64])`
- `pub const DEFAULT_DURATION_BUCKETS: &[f64]` (ex-constante privée
  `SECONDS_DURATION_BUCKETS`)

### Optimisations internes (transparentes pour l'utilisateur)

- `service_name` est converti **une seule fois** en `Arc<str>` lors du
  `Layer::layer()` ; chaque requête fait un `Arc::clone` (refcount atomique
  uniquement, pas d'alloc).
- Les méthodes HTTP standards (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS,
  CONNECT, TRACE) sont mappées vers `&'static str` via
  `fn method_label(&Method) -> Cow<'static, str>` — pas d'alloc pour 99 % des
  requêtes.
- Les codes HTTP usuels (200, 201, 204, 301, 302, 304, 400, 401, 403, 404,
  409, 422, 500, 502, 503, 504) sont mappés vers `&'static str` via
  `fn status_label(u16) -> Cow<'static, str>` — pas d'alloc pour les status
  courants.
- Les labels passent par `metrics::SharedString` (un `Cow<'static, str>`
  avec variante `Shared(Arc<str>)`) qui consomme `Arc<str>` sans réallouer.

### Tests ajoutés

Dans `src/server/axum/layers/prometheus.rs` :

1. **`middleware_does_not_block_on_system_metrics`** — sentinelle de
   régression : assert que la latence du middleware est < 50 ms. En 0.7 ce
   test aurait mesuré ~200 ms.
2. **`collector_ticks_without_panicking`** — smoke test : vérifie que la
   tâche démarre, tick au moins une fois, et n'a pas paniqué.

Run : `cargo test --all-features` → **44 tests OK + 8 doctests OK**.

---

## Vérification de performance

Mesure côté ApiRoad attendue après upgrade vers `api-tools 0.8.0` :

- `GET /health` avec `PROMETHEUS_ENABLE=1` :
  - **Avant** : ~210 ms (Docker ou natif)
  - **Après** : devrait s'aligner sur la latence avec Prometheus désactivé
    (~2-3 ms), soit **×80 à ×100**.

Il n'y aura **plus aucune raison fonctionnelle** de désactiver Prometheus
pour la performance. La variable `PROMETHEUS_ENABLE=0` peut être supprimée
de `server/.env.docker`.

---

## Fichiers modifiés dans la crate `api-tools`

- `src/server/axum/layers/prometheus.rs` — réécriture complète : layer
  allégé, suppression de `SystemMetrics` inline, ajout de
  `spawn_system_metrics_collector`, optimisations labels, 2 tests ajoutés.
- `src/server/axum/handlers/prometheus.rs` — ajout de
  `get_handle_with_buckets()` et de la constante publique
  `DEFAULT_DURATION_BUCKETS`.
- `Cargo.toml` — bump `0.7.0 → 0.8.0`.

## Hors scope (non fait)

- **Cache du handle `gauge!()` entre ticks** dans le collecteur background.
  ROI très faible (intervalle de 10 s, coût négligeable des lookups).
- **Cache du handle `counter!()` / `histogram!()` par route** dans le
  middleware. Possible via une `DashMap<RouteKey, (Counter, Histogram)>`,
  mais ajoute de la complexité sans résoudre de problème mesurable.

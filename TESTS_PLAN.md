# Tests à compléter

> État au 2026-05-07 : **56.88 % coverage** (310/545 lignes).
> Cible raisonnable à court terme : **≥ 75 %** sur les modules à 0 %, en
> commençant par les plus exposés (sécurité, observabilité).

Modules listés dans l'ordre d'exécution recommandé. Chaque entrée indique
les lignes non couvertes (cf. dernier rapport `cargo tarpaulin --all-features`),
les tests à écrire, et la difficulté.

---

## P0 — Critique (à faire en priorité)

### `server/axum/extractors.rs` — 0/36 (0 %)

Trois extracteurs Axum sans aucun test. `Path` et `Query` sont sur le hot
path de chaque handler.

- [ ] `ExtractRequestId` : header `x-request-id` valide → renvoie l'UUID.
- [ ] `ExtractRequestId` : header absent → `Rejection` (vérifier le code HTTP).
- [ ] `ExtractRequestId` : header présent mais non-UUID → comportement
      attendu (rejection ou empty selon le contrat).
- [ ] `Path` : path param valide (ex. `/users/:id`) deserialisé correctement.
- [ ] `Path` : type mismatch (ex. `:id` attendu en `u64`, reçu `"abc"`) → 400 + body JSON `ApiErrorResponse`.
- [ ] `Query` : query string valide → struct deserialisée.
- [ ] `Query` : query string vide / clés manquantes / type mismatch → 400.

**Setup** : monter un mini `axum::Router` dans le test, faire un `oneshot`
avec `Request::builder()`. Pattern déjà utilisé dans
`layers/prometheus.rs::tests`.

**Impact** : ~30 lignes couvrables. Difficulté : *moyenne* (Axum router
setup verbeux).

---

### `server/axum/layers/http_errors.rs` — 0/29 (0 %)

Middleware critique pour la cohérence des erreurs API. Aucun test.

- [ ] Service stub qui répond 404 → middleware réécrit le body en
      `ApiErrorResponse` JSON avec le bon `status` et `message`.
- [ ] Service stub qui répond 500 avec un body texte → réécriture en JSON.
- [ ] Service stub qui répond 200 → middleware ne touche pas la réponse.
- [ ] `Content-Type` de la réponse réécrite est bien
      `application/json`.
- [ ] Headers customs passés au layer sont attachés à la réponse d'erreur.

**Impact** : 29 lignes. Difficulté : *facile* (test unitaire classique).

---

### `server/axum/layers/security_headers.rs` — 0/27 (0 %)

- [ ] Avec `SecurityHeadersConfig::default()`, vérifier la présence de
      `Content-Security-Policy`, `X-Content-Type-Options: nosniff`,
      `X-Frame-Options`, `Referrer-Policy`, `Strict-Transport-Security`,
      `Permissions-Policy` (selon ce qui est configuré par défaut).
- [ ] Override d'un header via la config → la valeur custom est utilisée.
- [ ] Désactivation d'un header (si l'API le permet) → header absent.

**Impact** : 27 lignes. Difficulté : *facile*.

---

## P1 — Important

### `server/axum/layers/cors.rs` — 0/15 (0 %)

- [ ] `CorsConfig::default()` produit un `CorsLayer` qui répond aux
      preflight `OPTIONS` avec les bons `Access-Control-Allow-*`.
- [ ] `CorsConfig` avec origines whitelistées : requête depuis une origine
      autorisée passe, depuis une non-autorisée est rejetée.
- [ ] Méthodes/headers exposés conformes à la config.

**Impact** : 15 lignes. Difficulté : *facile* (mais attention aux subtilités
preflight).

---

### `server/axum/layers/logger.rs` — 4/44 (9 %)

Le middleware de logging n'a quasi aucun test. La nouvelle règle « skip
OPTIONS » (commit `76f81c7`) n'est pas testée.

- [ ] Capturer les logs avec `tracing-test` ou un subscriber custom :
  - [ ] Requête `GET /foo` → log de niveau INFO contenant méthode, path,
        status, latence.
  - [ ] Requête `OPTIONS /foo` → **aucun log** (régression sentinelle).
  - [ ] Requête qui retourne 5xx → log au niveau ERROR.
  - [ ] Requête qui retourne 4xx → log au niveau WARN (selon la convention
        actuelle).

**Impact** : 40 lignes. Difficulté : *moyenne* (capture des `tracing`
events demande un setup).

---

### `server/axum/handlers/prometheus.rs` — 0/7 (0 %)

- [ ] `PrometheusHandler::get_handle()` réussit en l'absence de recorder
      installé (premier appel d'un test isolé).
- [ ] `get_handle_with_buckets(&[0.001, 0.01, 0.1])` réussit avec des
      buckets custom.
- [ ] Double `install_recorder()` → erreur `ApiError::InternalServerError`
      (le recorder global ne peut être installé qu'une fois).

**Note** : ces tests doivent être dans un fichier de tests d'intégration
(`tests/`) et utiliser `#[serial_test::serial]` ou être exécutés avec
`--test-threads=1`, parce que `install_recorder()` mute un état global —
sinon ils interfèrent entre eux et avec les tests du module
`layers/prometheus`.

**Impact** : 7 lignes. Difficulté : *moyenne* (gestion du global state).

---

## P2 — Polish

### `server/axum/layers/time_limiter.rs` — 19/44 (43 %)

Le middleware lui-même n'est pas testé, seules `TimeSlots::contains/values`
le sont.

- [ ] Requête à l'intérieur d'un slot autorisé → handler invoqué.
- [ ] Requête en dehors → 503 Service Unavailable + body `ApiErrorResponse`.
- [ ] `TimeSlots::contains` aux bornes exactes (minute d'ouverture/de
      fermeture).
- [ ] Slot qui chevauche minuit (ex. 22:00 → 02:00).

**Impact** : 25 lignes. Difficulté : *facile*.

---

### `server/axum/layers/basic_auth.rs` — 26/36 (72 %)

Lignes manquantes : 86, 88-94, 97, 101 — probablement les branches
d'erreur (header malformé, base64 invalide, credentials wrong).

- [ ] Header `Authorization` absent → 401.
- [ ] Header présent mais pas `Basic <base64>` → 401.
- [ ] Base64 invalide → 401 (pas de panic).
- [ ] User/password correct → handler invoqué.
- [ ] User correct, password incorrect → 401.

**Impact** : 10 lignes. Difficulté : *facile*.

---

### `value_objects/pagination.rs` — 18/23 (78 %)

Lignes manquantes : 59, 63, 65, 94, 113. Probablement les bornes.

- [ ] `Pagination::new(0, 10)` ou `page = 0` → comportement attendu
      (panic ? clamp ?).
- [ ] `limit > PAGINATION_MAX_LIMIT` → clamp ou erreur.
- [ ] `total = 0` → `total_pages = 0` ou `1`.
- [ ] `set_max_limit` avec une valeur < `PAGINATION_MIN_LIMIT`.

**Impact** : 5 lignes. Difficulté : *triviale*.

---

## Hors scope (pour mémoire)

- `value_objects/datetime.rs` — 14/15 (93 %), une seule ligne manquante
  (probablement un cas `Display`).
- `server/axum/security/jwt/access_token.rs` — 9/12 (75 %), couvert
  raisonnablement après les ajouts récents.
- `server/axum/security/jwt/mod.rs` — désormais bien couvert par les
  tests P0 du précédent passage (round-trip, expiration, secrets erronés).

---

## Estimation globale

| Tier | Modules | Lignes couvrables | Difficulté |
|------|---------|-------------------|------------|
| P0 | extractors, http_errors, security_headers | ~86 | facile/moyenne |
| P1 | cors, logger, handlers/prometheus | ~62 | moyenne |
| P2 | time_limiter, basic_auth, pagination | ~40 | facile |

Si les trois tiers sont menés à bout, projection :
**56.88 % → ~85 %** (soit ~190 lignes supplémentaires couvertes sur
545).

---

## Méthodologie

1. **Un module à la fois.** PR par module, pas de big-bang.
2. **Tests in-file** (`#[cfg(test)] mod tests`) sauf pour les modules avec
   état global (ex. `handlers/prometheus.rs`) qui doivent passer en
   `tests/` avec sérialisation.
3. **Pattern existant** : reproduire le setup de
   `layers/prometheus.rs::tests` (mini service via `tower::service_fn` +
   `oneshot`) plutôt que monter un `axum::Router` complet quand c'est
   possible — moins de boilerplate.
4. **Sentinelles** : ajouter au moins un test « régression » par module
   (comme `middleware_does_not_block_on_system_metrics` pour Prometheus
   et le futur « OPTIONS-no-log » pour Logger) qui verrouille un
   comportement non-évident.
5. **Validation** : `cargo test --all-features && cargo clippy --all-features
   --all-targets -- -D warnings && cargo tarpaulin --all-features` avant
   chaque merge.

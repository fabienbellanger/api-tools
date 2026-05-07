# Tests à compléter

> État au 2026-05-07 (après hors-scope) : **84.56 % coverage** (460/544
> lignes, +27.68 pts vs. baseline 56.88 %).
> Cible initiale **≥ 75 %** largement dépassée ; cible étendue **≥ 85 %**
> à portée (plafond ~85 % imposé par l'instrumentation tarpaulin sur les
> corps `async move`, voir bilan en bas).

Modules listés dans l'ordre d'exécution recommandé. Chaque entrée indique
les lignes non couvertes (cf. dernier rapport `cargo tarpaulin --all-features`),
les tests à écrire, et la difficulté.

---

## P0 — Critique ✅ Terminé (2026-05-07)

17 tests ajoutés sur les trois modules. Couverture par module après
exécution :

| Module                                   | Avant      | Après        |
| ---------------------------------------- | ---------- | ------------ |
| `server/axum/extractors.rs`              | 0/36 (0 %) | 16/36 (44 %) |
| `server/axum/layers/http_errors.rs`      | 0/29 (0 %) | 22/29 (76 %) |
| `server/axum/layers/security_headers.rs` | 0/27 (0 %) | 18/27 (67 %) |

Lignes restantes :

- **extractors.rs** : branches de rejection profondes du `PathRejection`
  (`UnsupportedType`, `MissingPathParams`, `InvalidUtf8InPathParam`,
  `Message`, `WrongNumberOfParameters`) — difficiles à déclencher sans
  setups Axum très spécifiques. À faire dans un second passage si besoin.
- **http_errors.rs** : branches `audio/`, `video/`, `content_type` non
  ASCII, et l'erreur UTF-8 du body. Cas exotiques.
- **security_headers.rs** : tarpaulin ne couvre pas systématiquement le
  corps de l'`async move` (faux négatif d'instrumentation), mais les
  asserts confirment l'écriture des 7 headers.

### Tests écrits

- `extractors.rs::tests` — `RequestId` (présent / absent), `Path` (succès /
  type mismatch → 400 + JSON), `Query` (succès / clé manquante / type
  mismatch → 400).
- `layers/http_errors.rs::tests` — passthrough 200, réécriture 404 vide en
  JSON, passthrough 404 non-vide, réécriture 405 et 422 en JSON,
  short-circuit `image/png` (binaire intact), `PayloadTooLarge` quand le
  body dépasse `body_max_size`.
- `layers/security_headers.rs::tests` — defaults appliqués (7 headers),
  override custom + non-overridés conservés, écrasement d'une valeur déjà
  présente sur la réponse interne.

---

## P1 — Important ✅ Terminé (2026-05-07)

12 tests ajoutés sur les trois modules. Couverture par module :

| Module                               | Avant      | Après         |
| ------------------------------------ | ---------- | ------------- |
| `server/axum/layers/cors.rs`         | 0/15 (0 %) | 14/14 (100 %) |
| `server/axum/layers/logger.rs`       | 4/44 (9 %) | 24/44 (55 %)  |
| `server/axum/handlers/prometheus.rs` | 0/7 (0 %)  | 7/7 (100 %)   |

Lignes restantes dans `logger.rs` (98, 102-105, 107-119, 124, 132) : ce
sont essentiellement la définition du `macro_rules! log_request!` et les
sites d'expansion sous `info!` / `error!`, que tarpaulin n'instrumente pas
correctement (les tests passent par toutes les branches mais leur corps
est le résultat d'une expansion macro). Pas de capture `tracing` ajoutée
(éviterait l'ajout d'un dev-dep `tracing-subscriber` pour un gain de
couverture marginal).

### Tests écrits

- `layers/cors.rs::tests` — wildcard `*` → `Allow-Origin: *`, whitelist
  autorisée → origin renvoyée + `Allow-Credentials: true`, whitelist
  rejetée → header absent, chaîne vide → fallback `Any`.
- `layers/logger.rs::tests` — passthrough 2xx, 5xx (branche ERROR), 503
  traité comme INFO (cas `time_limiter`), **OPTIONS short-circuit
  (sentinelle commit 76f81c7)**, `/metrics` skip, headers optionnels
  absents.
- `handlers/prometheus.rs::tests` — `DEFAULT_DURATION_BUCKETS` strictement
  croissant ; lifecycle complet en un seul `#[test]` (premier install
  réussit, ré-installs renvoient `ApiError::InternalServerError` car le
  recorder global est figé).

**Note d'architecture** — le test `handler_install_recorder_lifecycle` est
gardé en in-file (`#[cfg(test)] mod tests`) plutôt qu'en `tests/`. Comme
c'est aujourd'hui le seul appel à `install_recorder()` dans la suite de
tests, l'état global ne crée aucune interférence ; déplacer en intégration
avec `serial_test` ne s'imposera que si un autre site appelle plus tard
`install_recorder()`.

---

## P2 — Polish ✅ Terminé (2026-05-07)

15 tests ajoutés sur les trois modules. Couverture par module :

| Module                               | Avant        | Après         |
| ------------------------------------ | ------------ | ------------- |
| `value_objects/pagination.rs`        | 18/23 (78 %) | 22/23 (96 %)  |
| `server/axum/layers/basic_auth.rs`   | 26/36 (72 %) | 26/36 (72 %)¹ |
| `server/axum/layers/time_limiter.rs` | 19/44 (43 %) | 36/44 (82 %)  |

¹ `basic_auth.rs` reste à 72 % côté tarpaulin malgré 5 nouveaux tests qui
exercent toutes les branches : les lignes restantes (86, 88-94, 97, 101)
sont à l'intérieur d'un `Box::pin(async move { … })`, que tarpaulin ne
sait pas systématiquement instrumenter — même symptôme observé sur
`security_headers.rs` (P0). Les tests passent et asserts confirment le
comportement.

Lignes restantes :

- **pagination.rs** : ligne 94 (branche `PAGINATION_DEFAULT_LIMIT >
PAGINATION_MAX_LIMIT` dans `Default::default()`) — code inatteignable
  tant que `200 < 500`. À noter comme dead code potentiel.
- **time_limiter.rs** : 170, 172-176, 178, 182 — corps `async move` du
  middleware (faux négatif tarpaulin).

### Tests écrits

- `value_objects/pagination.rs::test` — `new()` : page 0 → 1, limit < min
  → MIN, limit > max → MAX, max_limit > MAX clampé à MAX, custom max_limit
  respecté ; `PaginationResponse::new` (assignation des trois champs).
- `layers/basic_auth.rs::tests` — header absent → 401 + JSON +
  `WWW-Authenticate`, schéma non-Basic (Bearer) → 401, base64 invalide →
  401 sans panic, credentials valides → handler invoqué, password
  incorrect → 401.
- `layers/time_limiter.rs` : `tests` (in-file) — `contains` aux bornes
  (inclusif des deux côtés) et empty slots ; `middleware_tests` —
  empty slots → handler invoqué, slot couvrant `00:00-23:59` → 503 + JSON
  `ApiErrorResponse`.

---

## Hors scope ✅ Terminé (2026-05-07)

9 tests ajoutés sur les trois modules.

| Module                                     | Avant        | Après         |
| ------------------------------------------ | ------------ | ------------- |
| `value_objects/datetime.rs`                | 14/15 (93 %) | 15/15 (100 %) |
| `server/axum/security/jwt/access_token.rs` | 9/12 (75 %)  | 12/12 (100 %) |
| `server/axum/security/jwt/mod.rs`          | 40/73 (55 %) | 68/73 (93 %)  |

Lignes restantes dans `jwt/mod.rs` (117-118, 189, 191, 208) :

- 117-118 : branche `(_, Some(public_key), false)` de `init` — exigerait
  un couple PEM valide (clé privée et publique générées en test ou en
  fixture). Pas couvert pour éviter d'introduire un dev-dep
  cryptographique pour deux lignes.
- 189, 191, 208 : branches succès dans `match` sur `encoding_key.clone()` /
  `decoding_key.clone()` — tarpaulin ne les compte pas alors que
  `test_jwt_init_and_round_trip_hs512` les exerce (faux négatif similaire
  au pattern `async move`).

### Tests écrits

- `value_objects/datetime.rs::test` — `UtcDateTime::new()` (wrap d'un
  `DateTime<Utc>` existant) et `now()` (encadrement par `Utc::now()`).
- `jwt/access_token.rs::tests` — `FromRequestParts` succès (Bearer
  présent) et erreur (header absent → `ApiError::Unauthorized`).
- `jwt/mod.rs::tests` — `From<JwtError> for ApiError` (préserve le
  `Display`), getters/setters de lifetime, `set_encoding_key` /
  `set_decoding_key` rejettent un PEM invalide pour chaque algo non-HMAC
  (ES256/384, RS256/384/512, PS256/384/512, EdDSA), `init` ES256 +
  `private_key` invalide, `parse` sans `decoding_key`.

---

## Estimation globale

| Tier | Modules                                   | Lignes couvrables | Difficulté     |
| ---- | ----------------------------------------- | ----------------- | -------------- |
| P0   | extractors, http_errors, security_headers | ~86               | facile/moyenne |
| P1   | cors, logger, handlers/prometheus         | ~62               | moyenne        |
| P2   | time_limiter, basic_auth, pagination      | ~40               | facile         |

P0 livré : **56.88 % → 67.16 %** (+56 lignes).
P1 livré : **67.16 % → 74.82 %** (+41 lignes).
P2 livré : **74.82 % → 78.68 %** (+21 lignes).
Hors-scope livré : **78.68 % → 84.56 %** (+32 lignes).

**Total** : +150 lignes couvertes, +27.68 pts, **53 tests ajoutés** sur
quatre passages.

Le plafond actuel (~80 %) est principalement dû au pattern d'instrumentation
de tarpaulin sur les corps `async move` (cf. `security_headers.rs`,
`basic_auth.rs`, `logger.rs` macro_rules, `time_limiter.rs`). Aller au-delà
demanderait soit un autre outil de couverture (e.g. `cargo llvm-cov`), soit
de refactorer les middlewares pour extraire la logique du closure async.

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

# Plan — `HttpErrorsLayer` : early-return avant bufferisation

> Statut : **implémenté** (v0.9.1). Origine : audit backend apiroad, item **H2**.

## Problème

`server::axum::layers::http_errors::HttpErrorsMiddleware::call` ne réécrit que trois
statuts (`404` vide, `405`, `422`), mais il **bufferise et copie le corps de _toutes_ les
réponses** avant même de regarder le statut.

Séquence actuelle (`src/server/axum/layers/http_errors.rs`, ~l.80-94) :

```
inner.call()
  -> early-return si content-type image/audio/video
  -> into_parts()
  -> axum::body::to_bytes(body, max)     <-- bufferisation COMPLETE, meme pour un 200 OK
  -> String::from_utf8(bytes.to_vec())   <-- copie memoire + scan UTF-8
  -> match parts.status { 405 | 422 | 404-vide => reecrit ; _ => passthrough }
```

Conséquences pour un consommateur (ex. apiroad, layer appliqué à tout `/api/v1`) :

1. **Allocation + copie + validation UTF-8** proportionnelles à la taille de **chaque**
   réponse JSON, y compris les `200 OK` qui forment l'essentiel du trafic.
2. **Streaming neutralisé** : le TTFB attend que le corps entier soit prêt.
3. **Faux `413 PayloadTooLarge`** : un corps légitime qui dépasse `body_max_size` fait
   échouer `to_bytes` et transforme un `200` en `413` injustifié.

Le corps n'est réellement nécessaire que pour `422` (réémis) et `404` (test de vacuité).
Le `405` **jette** son corps. Tous les autres statuts sont de simples passthrough.

## Correctif proposé

Inverser l'ordre : matcher `parts.status` **d'abord**, ne bufferiser que pour `422`/`404`,
et laisser passer tout le reste (dont `200` et `405`) sans toucher au `Body`.

```rust
let (parts, body) = response.into_parts();

// Only 404/405/422 are rewritten. Every other status (notably the 200s that make up
// the bulk of the traffic) is returned untouched, without buffering: this preserves
// streaming and avoids a full-body copy + UTF-8 scan, plus the spurious 413 a legitimate
// large body used to trigger by overflowing `to_bytes`.
match parts.status {
    // The 405 body is discarded, so there is no need to read it.
    StatusCode::METHOD_NOT_ALLOWED => Ok(ApiError::MethodNotAllowed.into_response()),
    // 422/404 are the only statuses whose body we still need to inspect.
    StatusCode::UNPROCESSABLE_ENTITY | StatusCode::NOT_FOUND => {
        match axum::body::to_bytes(body, config.body_max_size).await {
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Ok(body) => match parts.status {
                    StatusCode::UNPROCESSABLE_ENTITY => Ok(ApiError::UnprocessableEntity(body).into_response()),
                    StatusCode::NOT_FOUND if body.is_empty() => {
                        Ok(ApiError::NotFound("Resource Not Found".to_owned()).into_response())
                    }
                    _ => Ok(Response::from_parts(parts, Body::from(body))),
                },
                Err(err) => Ok(ApiError::InternalServerError(err.to_string()).into_response()),
            },
            Err(_) => Ok(ApiError::PayloadTooLarge.into_response()),
        }
    }
    _ => Ok(Response::from_parts(parts, body)),
}
```

### Notes de conception

- **Le check content-type image/audio/video est conservé, en tête**, pour garder le
  comportement exact : un média n'est jamais réécrit quel que soit son statut. Avec le
  nouveau code il retomberait de toute façon sur le passthrough `_` pour un `200`, mais on
  ne change pas le cas limite d'un média en `404`/`422`.
- **`405` ne bufferise plus** : son corps était systématiquement lu puis jeté.
- **Aucun changement d'API** : signature, config et statuts réécrits identiques. C'est un
  correctif interne au `call`, donc rétro-compatible (patch SemVer `0.9.1`).

## Comportement — invariants à préserver

| Réponse in                          | Sortie attendue                          |
| ----------------------------------- | ---------------------------------------- |
| `200 OK` + corps                    | passthrough inchangé (désormais sans buffer) |
| `405`                               | `ApiError::MethodNotAllowed` (JSON)      |
| `422` + corps                       | `ApiError::UnprocessableEntity(body)`    |
| `404` corps vide                    | `ApiError::NotFound("Resource Not Found")` |
| `404` corps non vide                | passthrough inchangé                     |
| `500`, `3xx`, etc.                  | passthrough inchangé                     |
| content-type image/audio/video      | passthrough inchangé (early-return)      |
| corps > `body_max_size` sur `422`   | `413 PayloadTooLarge` (inchangé)         |
| **corps > `body_max_size` sur `200`** | **passthrough (avant : faux `413`)** — changement voulu |

## Tests à ajouter (dans `#[cfg(test)] mod tests`)

Les tests existants (`ok_response_passes_through_unchanged`, `empty_404_...`,
`non_empty_404_...`, `method_not_allowed_...`) doivent rester verts. Ajouter :

1. `large_ok_body_is_not_truncated_to_413` — un `200` dont le corps dépasse
   `body_max_size` doit ressortir `200` intact (garde anti-régression du faux `413`).
2. `passthrough_status_body_is_not_buffered` — un statut non réécrit (`500`) avec corps
   ressort inchangé.
3. (Optionnel) `unprocessable_entity_over_limit_still_413` — confirme que la limite reste
   appliquée là où le corps est réellement lu.

## Validation

```bash
cargo test --all-features -- --nocapture
make lint          # clippy -D warnings + fmt
make verify-msrv   # MSRV 1.88
```

## Release & consommation côté apiroad

1. Bump `Cargo.toml` en `0.9.1`, entrée `CHANGELOG.md`.
2. `make prepare` puis publication crates.io (process de release habituel).
3. Dans apiroad : passer `api-tools = { version = "0.9.1", ... }`, `cargo update -p api-tools`,
   `make lint && make test`, puis `make sqlx-prepare` si nécessaire.

> Alternative sans publication immédiate : `[patch.crates-io] api-tools = { git = "…", branch = "…" }`
> dans le `Cargo.toml` d'apiroad, le temps de valider en conditions réelles.

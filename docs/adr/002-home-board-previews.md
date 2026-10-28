# Home board previews

Status: accepted, 2026-10-28.

Home represents games as board tiles while preserving the separate Ongoing games and Review games cards. Rendering a full interactive Svelte board for every tile would multiply DOM nodes and client work, undermining the navigation performance fix.

Serve a compact, noninteractive SVG from `GET /api/games/{id}/preview.svg?head={active_head}&v=1`. The backend reconstructs the requested immutable position using the existing rules engine, including captures, and adds a last-move marker. No analysis or model is required. The frontend uses one lazy, asynchronously decoded image per tile and reserves its square dimensions.

The cache key is game ID plus move-node ID, never `updated_at`, selected review node or analysis state. A new active head creates a new image URL; existing URLs always retain their original board. Review-only variations and analysis updates therefore do not rebuild or reload a preview. Finished and imported games use their stored active continuation.

The browser receives a private, one-year immutable cache lifetime and a content ETag; conditional GETs return 304. SVG representation changes require bumping the frontend `v` parameter. A process-local LRU holds at most 128 images and can be discarded at restart. It is derived data, so no database migration or new source of truth is needed. Cold builds run on the blocking pool and are deduplicated by the cache mutex; cache hits avoid SQLite and rules replay.

A release benchmark on the reported 337-node game measured a median 0.607ms for rules replay plus SVG construction/hash over 50 iterations. The SVG was 10,684 bytes. This is small enough to update on every accepted active move when Home requests it; the suggested ten-move delay is unnecessary. Images are generated on demand, not eagerly after every move. No live game was changed by the benchmark.

Tests cover captures, requested-head consistency, branch isolation, bounded cache eviction, content types/ETags/304, active-head URL changes, and real browser image-cache reuse. The existing long-review navigation performance regression remains in place.

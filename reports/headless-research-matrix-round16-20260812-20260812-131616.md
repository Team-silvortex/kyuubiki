# Headless research matrix
- 时间: 2026-08-12 13:16:28 +0800
- 执行模式: dry=true mock=true service=true
- 模板源: custom
- 输出目录: /Users/Shared/chroot/dev/kyuubiki/results/headless-research-matrix-20260812-round16
- Service 主/兜底: http://127.0.0.1:3000 / http://127.0.0.1:4000, fallback=1

| template | init | validate | plan | render | validate_ok | validate_issue_count | dry_exit | dry_status | dry_mode | dry_error | mock_exit | mock_status | mock_mode | mock_error | service_primary_exit | service_primary_status | service_primary_mode | service_primary_api | service_primary_error | service_fallback_exit | service_fallback_status | service_fallback_mode | service_fallback_api | service_fallback_error | winner_id | winner_score | winner_field(V/m) | winner_safety |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct_mesh_pipeline | 0 | 0 | 0 | 0 | true | 0 | 0 | ok | dry_run | n/a | 0 | ok | execute:mock | n/a | 0 | ok | execute:service | http://127.0.0.1:3000 | n/a | -1 | skipped | skipped | http://127.0.0.1:4000 | n/a | n/a | n/a | n/a | n/a |
| material_heat_spreader_screening | 0 | 0 | 0 | 0 | true | 0 | 0 | ok | dry_run | n/a | 0 | ok | execute:mock | n/a | 0 | ok | execute:service | http://127.0.0.1:3000 | n/a | -1 | skipped | skipped | http://127.0.0.1:4000 | n/a | n/a | n/a | n/a | n/a |
| material_study_envelope_catalog | 0 | 0 | 0 | 0 | true | 0 | 0 | blocked | dry_run | n/a | -1 | skipped | skipped | n/a | -1 | skipped | skipped | http://127.0.0.1:3000 | n/a | -1 | skipped | skipped | http://127.0.0.1:4000 | n/a | n/a | n/a | n/a | n/a |
| material_study_envelope_ranking | 0 | 0 | 0 | 0 | true | 0 | 0 | blocked | dry_run | n/a | -1 | skipped | skipped | n/a | -1 | skipped | skipped | http://127.0.0.1:3000 | n/a | -1 | skipped | skipped | http://127.0.0.1:4000 | n/a | n/a | n/a | n/a | n/a |
| material_dielectric_screening | 0 | 0 | 0 | 0 | true | 0 | 0 | ok | dry_run | n/a | 0 | ok | execute:mock | n/a | 0 | ok | execute:service | http://127.0.0.1:3000 | n/a | -1 | skipped | skipped | http://127.0.0.1:4000 | n/a | polyimide_film | 0.8 | 30000.0 | 10000.0 |

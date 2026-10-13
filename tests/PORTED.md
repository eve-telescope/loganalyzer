# Ported tests

Every test of the Laravel app (`main`: `tests/Feature`, `tests/Unit`,
Pest, 43 tests) and the test that replaces it. The parser tests run the
Rust parser on the same logs (each inline log of the PHP test is a file in
`tests/fixtures/parser`); the upload tests drive the real router on the
test database with a mock ESI on an ephemeral port. Every assertion is
made exact: full key sets, exact messages, the names ESI was asked for.

| PHP test | New test |
| --- | --- |
| Feature/CombatLogParserTest "it extracts the listener name from the header" | `parser::it_extracts_the_listener_name_from_the_header` |
| Feature/CombatLogParserTest "it extracts the session start time" | `parser::it_extracts_the_session_start_time` |
| Feature/CombatLogParserTest "it parses combat events" | `parser::it_parses_combat_events` |
| Feature/CombatLogParserTest "it parses outgoing damage hits correctly" | `parser::it_parses_outgoing_damage_hits_correctly` |
| Feature/CombatLogParserTest "it parses incoming miss lines" | `parser::it_parses_incoming_miss_lines` |
| Feature/CombatLogParserTest "it parses outgoing miss lines" | `parser::it_parses_outgoing_miss_lines` |
| Feature/CombatLogParserTest "it handles empty combat logs gracefully" | `parser::it_handles_empty_combat_logs_gracefully` |
| Feature/CombatLogParserTest "it parses logistics received events" | `parser::it_parses_logistics_received_events` |
| Feature/CombatLogParserTest "it parses logi events from the test log file" | `parser::it_parses_logi_events_from_the_test_log_file` |
| Feature/CombatLogParserTest "it parses incoming energy neutralization events" | `parser::it_parses_incoming_energy_neutralization_events` |
| Feature/CombatLogParserTest "it parses incoming energy nosferatu events with negative GJ" | `parser::it_parses_incoming_energy_nosferatu_events_with_negative_gj` |
| Feature/CombatLogParserTest "it parses neutralization events from the testlog2 fixture" | `parser::it_parses_neutralization_events_from_the_testlog2_fixture` |
| Feature/CombatLogParserTest "events use enums for direction and type" | `parser::events_use_enums_for_direction_and_type` |
| Feature/CombatLogParserTest "it attributes outgoing drone damage to the sole owner" | `parser::it_attributes_outgoing_drone_damage_to_the_sole_owner` |
| Feature/CombatLogParserTest "it leaves drone damage unattributed when several pilots own that drone type" | `parser::it_leaves_drone_damage_unattributed_when_several_pilots_own_that_drone_type` |
| Feature/CombatLogParserTest "it strips markup from logistics dealt weapons" | `parser::it_strips_markup_from_logistics_dealt_weapons` |
| Feature/CombatLogParserTest "it parses multiline rich-format logistics dealt events" | `parser::it_parses_multiline_rich_format_logistics_dealt_events` |
| Feature/CombatLogParserTest "no parsed field ever contains markup" | `parser::no_parsed_field_ever_contains_markup` |
| Feature/CombatLogParserTest "it parses logistics received with a plain pilot target (issue #1)" | `parser::it_parses_logistics_received_with_a_plain_pilot_target_issue_1` |
| Feature/CombatLogParserTest "it parses rich-format outgoing energy neutralization with pilot details" | `parser::it_parses_rich_format_outgoing_energy_neutralization_with_pilot_details` |
| Feature/CombatLogParserTest "it parses rich-format outgoing nosferatu drains with positive GJ" | `parser::it_parses_rich_format_outgoing_nosferatu_drains_with_positive_gj` |
| Feature/CombatLogParserTest "it parses incoming neutralization with fontsize-wrapped ship names" | `parser::it_parses_incoming_neutralization_with_fontsize_wrapped_ship_names` |
| Feature/CombatLogParserTest "it parses outgoing neutralization from the testlog3 fixture" | `parser::it_parses_outgoing_neutralization_from_the_testlog3_fixture` |
| Feature/CombatLogUploadTest "the upload page renders with the configured max upload size" | `combat_log_upload::the_upload_page_renders_with_the_configured_max_upload_size`, `frontend/e2e/upload.spec.ts` "tells the limit of the upload" |
| Feature/CombatLogUploadTest "a combat log can be uploaded and redirects to show page" | `combat_log_upload::a_combat_log_can_be_uploaded_and_redirects_to_show_page`, `frontend/e2e/upload.spec.ts` "uploads a log and opens its analysis" |
| Feature/CombatLogUploadTest "the show page passes URL filters as a prop" | `combat_log_upload::the_show_page_passes_url_filters_as_a_prop` |
| Feature/CombatLogUploadTest "the show page returns empty filters when no query params are present" | `combat_log_upload::the_show_page_returns_empty_filters_when_no_query_params_are_present` |
| Feature/CombatLogUploadTest "the show page renders with analysis data" | `combat_log_upload::the_show_page_renders_with_analysis_data` |
| Feature/CombatLogUploadTest "uploading without a file returns validation error" | `combat_log_upload::uploading_without_a_file_returns_validation_error` |
| Feature/CombatLogUploadTest "uploading an oversized file returns validation error" | `combat_log_upload::uploading_an_oversized_file_returns_validation_error` |
| Feature/CombatLogUploadTest "combat events are stored as individual rows" | `combat_log_upload::combat_events_are_stored_as_individual_rows` |
| Feature/CombatLogUploadTest "the log is shareable via uuid url" | `combat_log_upload::the_log_is_shareable_via_uuid_url` |
| Feature/CombatLogUploadTest "uploading resolves pilot and ship names to EVE ids via ESI" | `combat_log_upload::uploading_resolves_pilot_and_ship_names_to_eve_ids_via_esi` |
| Feature/CombatLogUploadTest "the show page passes resolved EVE ids as props" | `combat_log_upload::the_show_page_passes_resolved_eve_ids_as_props` |
| Feature/CombatLogUploadTest "names already resolved are not requested from ESI again" | `combat_log_upload::names_already_resolved_are_not_requested_from_esi_again` |
| Feature/CombatLogUploadTest "uploading still succeeds when ESI is unavailable" | `combat_log_upload::uploading_still_succeeds_when_esi_is_unavailable` |
| Feature/CombatLogUploadTest "names matching an item type are never treated as characters" | `combat_log_upload::names_matching_an_item_type_are_never_treated_as_characters` |
| Feature/CombatLogUploadTest "uploading stores the compressed raw log and it can be downloaded" | `combat_log_upload::uploading_stores_the_compressed_raw_log_and_it_can_be_downloaded`, `frontend/e2e/analysis.spec.ts` "downloads the raw log as it was uploaded" |
| Feature/CombatLogUploadTest "the download route returns 404 when no raw log is stored" | `combat_log_upload::the_download_route_returns_404_when_no_raw_log_is_stored` |
| Feature/CombatLogUploadTest "a stored combat log can be regenerated from its raw contents" | `combat_log_upload::a_stored_combat_log_can_be_regenerated_from_its_raw_contents`, `frontend/e2e/analysis.spec.ts` "regenerates the analysis from the raw log" |
| Feature/CombatLogUploadTest "regeneration returns 404 without changing events when the raw log is missing" | `combat_log_upload::regeneration_returns_404_without_changing_events_when_the_raw_log_is_missing` |
| Feature/ExampleTest "returns a successful response" | `example::returns_a_successful_response`, `frontend/e2e/routes.spec.ts` "serves the two pages" |
| Unit/ExampleTest "that true is true" | dropped: asserts nothing about the app |

## New tests

- The parser against PHP (`parser::every_fixture_parses_like_php`,
  `parser::the_generated_corpus_parses_like_php`): the full output for the
  three repository logs, every inline log of the PHP tests, 32 edge cases
  (invalid UTF-8, UTF-16 chat logs, BOM, CRLF and bare CR, markup in
  names, `&lt;TICKER&gt;`, signs of GJ, huge numbers, every branch of the
  drone attribution) and 400 generated logs, compared with what the PHP
  parser answered. `scripts/parser-diff` runs the same comparison over
  freshly generated logs or a directory of raw logs.
- PHP's string functions (`php_functions`): `strip_tags`, `mb_trim`, the
  `(int)` cast, Symfony's client file name and extension and the
  download's `Content-Disposition`, against PHP's answers for 732 inputs.
- Golden comparisons with the Laravel reference: every page's props
  (`golden_pages`), every upload and regenerate scenario with the stored
  rows and the names ESI was asked for (`golden_mutations`), the error
  pages, downloads and static files (`golden_http`).
- The route contract (`routes`, `frontend/e2e/routes.spec.ts`), the ESI
  client (`esi_client`), the URL state (`filters`), the uuid forms, the
  request limit, the layers and the sample seed (`behavior`).
- The analysis math (`frontend/src/lib/analysis.test.ts`): every value the
  Laravel page computed (buckets, summary, details, peaks, tables, chart
  items, the engagement dialog, the OpenGraph description, avatars, the
  hit quality bars) for 17 logs and URL states, compared with the outputs
  of the legacy `Analysis.vue` script itself
  (`scripts/laravel-reference/analysis-outputs.mjs`).
- The pages in a browser (`frontend/e2e`): the upload flow with progress
  and both size checks, a shared link with all four parameters, the drag
  selection, the series toggles, the pilot dialog, the tabs, download,
  regeneration, the state without a raw file, the heads against Laravel's
  (`seo.spec.ts`), the visible text of the analysis pages against
  Laravel's (`analysis-text.spec.ts`), screenshots against Laravel's
  (`visual.spec.ts`, not in CI) and no JavaScript errors (`smoke.spec.ts`).

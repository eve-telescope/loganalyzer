# Fixes

Every Laravel bug the rewrite fixes instead of copying, with the test that
pins the new behavior, then the deliberate changes and what is left for
the owner to decide. Together with the test normalizations N1 to N5
(`tests/integration/common/golden.rs`) this is the complete list of
differences from the Laravel app on `main`.

## Bugs fixed

### Some logs failed the upload with 500

Laravel: four kinds of combat lines crashed the parser, and with it the
whole upload:

- a rich energy line (the listener neutralizing or draining someone)
  whose target block had no `<b>` or `<fontsize>` ship markup: PHP
  warned "Undefined array key 1", which Laravel turned into an exception;
- a rich energy line whose `[Pilot]` brackets sat inside a tag, so
  `strip_tags` removed them: the pilot was null and the parser refused it
  (TypeError);
- a drone owner whose name was a number (`12345`): PHP made the name an
  integer array key and the rebuilt event refused it (TypeError);
- an energy amount below `PHP_INT_MIN`: `abs()` returned a float and the
  event refused it (TypeError).

Now: the line without ship markup keeps its event with no ship; the line
whose pilot sits in markup is read as the plain energy format (the next
pattern, as if the rich one had not matched); numeric names stay names;
amounts saturate at `PHP_INT_MAX`, as `(int)` already did.

Tests: `parser::a_rich_energy_line_without_a_ship_keeps_the_event`,
`parser::a_rich_energy_line_whose_pilot_sits_in_markup_reads_as_the_plain_format`,
`parser::a_drone_owner_with_a_numeric_name_is_attributed`,
`parser::an_energy_amount_below_php_int_min_saturates`,
`golden_mutations::upload_numeric_pilot`,
`golden_mutations::upload_rich_neut_without_ship`.

### Long names and big numbers failed the upload with 500

Laravel: every text column was `varchar(255)` and `damage` was
`int unsigned`, so a log with a field over 255 characters or a hit over
4294967295 failed the insert, after the log row and the raw file were
already written.

Now: the text columns are `text` and `damage` is `bigint` (never
negative), so whatever the parser makes of a log is stored.

Tests: `golden_mutations::upload_long_fields`,
`golden_mutations::upload_huge_numbers`.

### An upload could leave a log without its events

Laravel: the log row and the raw file were written first and the events
inserted afterwards, outside a transaction, so a failure in between left
a log with no or some events.

Now: the raw file is written first, then the log and all its events in
one transaction; when that fails, the raw file is removed again and the
upload answers 500.

Test: `behavior::a_failed_save_leaves_no_raw_file`.

### A ticker or ship called "0" was dropped

Laravel: PHP's `?:` treats the string `"0"` as empty, so a corporation
ticker `[0]` became null, and a rich energy line's ship `0` became no
ship. The analysis page's name lookup also compared names loosely
(`"1" == "01"` in PHP), so of two numeric-looking names only one could
get its id.

Now: only the empty string is empty, and names compare exactly.

Tests: `parser::a_corporation_or_ship_named_0_is_kept`,
`parser::every_fixture_parses_like_php` (the two fixtures are listed in
its `known_difference`).

### A damaged raw log answered 500

Laravel: `gzdecode()` warns on data that is not gzip, which Laravel turns
into an exception before the controller's own check, so regenerating
answered 500 instead of the intended 422 "The stored combat log could not
be decoded.", and the download answered 500 with a cut-off response.

Now: regenerating answers 422 with that message (and changes nothing),
the download answers the 422 error page.

Tests: `golden_mutations::regenerate_corrupt`,
`golden_http::download_corrupt`.

### Array query parameters broke the analysis page

Laravel: `?hide[]=x` answered 500 ("Array to string conversion"), and
`?pilot[]=x` or `?from[x]=1` passed an array into the page, which then
treated it as a pilot name or a time.

Now: a parameter sent as an array is ignored (null, or no hidden
series). Bytes in a parameter that are not UTF-8 become U+FFFD instead of
failing the JSON encoding.

Tests: `golden_pages::log_filters_hide_array`,
`golden_pages::log_filters_array`,
`filters::array_parameters_are_ignored`,
`filters::bytes_that_are_not_utf8_become_replacement_characters`.

### ESI being down held an upload for 20 seconds

Laravel: each chunk of names was tried five times, five seconds apart,
on a server error; a connection that hung was waited for with no limit
of its own, and a connection error was not retried at all. A success
whose body was not ESI's documented shape crashed the upload (TypeError).

Now: two tries, 250 ms apart, each limited to five seconds, for server
errors, connection errors and timeouts alike; a chunk that still fails,
or whose answer is malformed, is skipped and its names stay unresolved,
as before. All of it is configurable (`ESI_TRIES`,
`ESI_RETRY_DELAY_MS`, `ESI_TIMEOUT_MS`).

Tests: `golden_mutations::upload_esi_down`,
`esi_client::retries_a_server_error_once`,
`esi_client::gives_up_after_two_tries_and_leaves_the_names_uncached`,
`esi_client::does_not_retry_a_client_error`,
`esi_client::an_unreachable_esi_costs_two_quick_tries`,
`esi_client::a_slow_answer_times_out_and_is_retried`,
`esi_client::an_answer_that_is_not_the_documented_shape_skips_the_chunk`.

### A log just under the limit got nginx's error page

Laravel: nginx capped the whole request at 25 MiB, multipart framing
included, while Laravel allowed a 25600 KiB file, so a file just under
the limit got nginx's bare 413 page (in Inertia's error modal) instead of
the validation message.

Now: the request limit is the file limit plus 1 MiB (the api's and
Caddy's), and a body the api still refuses for its size gets the size
message as a 422. The upload form shows the same message for a 413 from
the proxy.

Tests: `behavior::a_body_over_the_request_limit_gets_the_size_message`,
`combat_log_upload::uploading_an_oversized_file_returns_validation_error`,
`golden_mutations::upload_max_size`,
`frontend/src/lib/upload.svelte.test.ts` "turns a refused body into the
size message".

### A link with a space after the uuid opened the log

Laravel: MySQL ignores trailing spaces when comparing (`PAD SPACE`), so
`/logs/{uuid}%20` showed the log.

Now: a log is found by its uuid in the canonical 8-4-4-4-12 form, in any
case, and nothing else (braces and the form without hyphens, which
Postgres would accept, stay 404 as before).

Tests: `golden_pages::log_trailing_space`,
`behavior::a_log_is_found_by_its_uuid_in_any_case_only_in_the_canonical_form`.

## Deliberate changes

### The upload and the regeneration answer JSON

The pages send the upload with XMLHttpRequest (for the progress bar;
fetch cannot report upload progress) and the regeneration with fetch. A
success answers `200 {"redirect": "/logs/{uuid}"}` instead of Laravel's
302, a failed validation `422 {"errors": {"log_file": message}}` instead
of a redirect back with the errors flashed (normalization N4), other
failures `{"message": ...}` with their status. A regeneration that does
not ask for JSON gets the error page Laravel showed.

Tests: `combat_log_upload` (all), `golden_mutations` (all),
`frontend/src/lib/upload.svelte.test.ts`.

### Failures show in the page

Laravel showed a failed upload or regeneration (413, 500) in Inertia's
error modal with the raw error page. The upload form now shows the size
message for a 413 and "The log file failed to upload." for anything
else under the drop zone; a failed regeneration shows its message under
the header.

Tests: `frontend/src/lib/upload.svelte.test.ts`,
`frontend/e2e/upload.spec.ts` "shows the server validation message".

### A same-origin check instead of the CSRF token

Without sessions there is no CSRF token. A mutation whose
`Sec-Fetch-Site` is not `same-origin` or `none`, or whose `Origin` names
another host, is refused with 403 (Laravel answered 419 for a missing
token). A request without either header (a script, curl) is not a
cross-site request and passes; nothing but the upload form posts to
`/analyze` today.

Tests: `golden_mutations::upload_without_csrf`,
`golden_mutations::regenerate_without_csrf`,
`frontend/e2e/routes.spec.ts` "refuses an upload from another site".

### One URL per page

Laravel answered `/logs/{uuid}/` and `/index.php` like the bare path. A
trailing slash now redirects to the path without it (308, which keeps the
method of a post), and `/index.php` (also as a prefix,
`/index.php/logs/...`) to the path after it (301). The plain http to https
redirect is Caddy's 308 instead of nginx's 301; browsers follow both.

Tests: `golden_pages::log_trailing_slash`, `golden_pages::index_php`,
`golden_http::download_trailing_slash`,
`behavior::trailing_slashes_and_the_front_controller_redirect`,
`frontend/src/lib/server/canonical.test.ts`,
`frontend/e2e/routes.spec.ts` "redirects trailing slashes and the front
controller".

### Nothing is served from /storage

Laravel registered its local disk routes (`GET` and `PUT
/storage/{path}`), which answered 404 without a signed URL the app never
made. Every `/storage` path answers 404 now.

Tests: `golden_http::storage_get`, `golden_http::storage_put`,
`routes::every_laravel_route_is_answered`.

### The URL state changes without a server round trip

Laravel sent every change of the selected range, the hidden series or the
open pilot to the server (an Inertia visit for `filters`) to rewrite the
URL. The page now rewrites it in place (`replaceState`), in the same
order (`from`, `to`, `hide` in series order, `pilot`), and like before
not on the first render, so a shared link stays as it was until the
visitor changes something.

Tests: `frontend/src/lib/analysis.test.ts` "the URL state",
`frontend/e2e/analysis.spec.ts`.

### Regenerating keeps the view

Laravel redirected to the bare log URL after regenerating, which dropped
the selected range and the open pilot. The page now reloads its data in
place and keeps them.

Test: `frontend/e2e/analysis.spec.ts` "regenerates the analysis from the
raw log".

### Logs without a raw file say so

About 200 logs were uploaded before raw files were kept. Laravel showed
neither the download link nor the regenerate button for them; the header
now says "No raw log" (with a tooltip why) in their place.

Test: `frontend/e2e/analysis.spec.ts` "a log without a raw file says so
and offers neither download nor regeneration".

### The ESI client names itself

Laravel sent `User-Agent: EVE Log Analyzer` (APP_NAME). ESI asks clients
to name themselves and a way to reach the owner: the api sends `EVE Log
Analyzer/1.0 (+https://loganalyzer.eve-telescope.com;
+https://github.com/eve-telescope/loganalyzer)`, `ESI_USER_AGENT`
overrides it. The compatibility date (`2026-05-19`) and the names asked
for are unchanged (normalization N5).

Tests: `esi_client::sends_the_names_with_the_user_agent_and_compatibility_date`,
`esi_client::the_default_user_agent_names_the_site_and_its_source`.

### The props of the pages

The api serves each page's props as JSON (`/api/pages/home`,
`/api/pages/logs/{uuid}`) with the shared props `name` and `auth`.
Inertia's `errors` bag is gone (N1), an empty `pilotIds` or
`shipTypeIds` is `{}` instead of PHP's `[]` (N3), and the events come in
the order they were logged (`order by id`; MySQL returned index order
without being asked). Requests from tabs left open on the Laravel site
(`X-Inertia: true`) get a 409 with `X-Inertia-Location`, so Inertia
reloads them as full pages.

Tests: `golden_pages` (all), `behavior::events_come_in_the_order_they_were_logged`,
`behavior::open_inertia_tabs_reload_the_page`,
`frontend/src/lib/server/inertia.test.ts`.

### Downloads carry their length

Laravel streamed the raw log without a `Content-Length`; the download now
has one. The type, the cache header and the file name are as before.

Tests: `behavior::a_download_without_a_file_name_is_combat_log_txt`,
`golden_http::download_testlog`.

### The ASCII fallback name of downloads

The `filename*` of a download (what browsers use) is unchanged. The plain
`filename` fallback was Laravel's `Str::ascii` transliteration; it is now
`deunicode` for Latin, Greek and Cyrillic letters, Unicode spaces as
spaces, and other characters dropped. Of the recorded names only `ǅ.txt`
differs (`Dz.txt` instead of `.txt`).

Tests: `php_functions::the_download_disposition_answers_like_symfony`,
`golden_http::download_multiline`.

### The regular expression engine's limits

The parser keeps PHP's patterns on PCRE2 with PHP's options, but the
bundled PCRE2 (10.46) applies its own match limit of 10,000,000 steps
where PHP stopped at `pcre.backtrack_limit` (1,000,000) and treated that
as no match. No fixture and no generated log comes near either limit.

Tests: `parser::every_fixture_parses_like_php`,
`parser::the_generated_corpus_parses_like_php`, `scripts/parser-diff`.

## Needs owner decision

### A contact in the ESI User-Agent

ESI's guidelines ask for a way to reach the owner (an email or a
profile) in the User-Agent. The default names the site and the GitHub
repository; set `ESI_USER_AGENT` in production with the contact of your
choice.

### The differential parse over production logs

The parser is checked against PHP over the repository's fixtures, edge
cases and generated logs (`scripts/parser-diff`, which prints counts and
names only). It can run over the stored raw logs on the new box at C4
(`scripts/parser-diff /path/to/combat-logs`) once that is approved.

### Name collation at import

MySQL's `utf8mb4_unicode_ci` and the ICU collation of `eve_entities.name`
(`und-u-ks-level1`) both ignore case and accents, but differ in details
(MySQL also ignores trailing spaces). The C4 rehearsal should check that
no two imported names collide or split under the new unique key.

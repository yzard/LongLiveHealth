# Unified API and shared Strata inference

Long Live Health now deploys one backend_api business service on NAS. Rust owns page preparation, the retained laboratory-page-v5 prompt, shared HTTP inference, structured-output checks, draft persistence, review and Analytics. The independent Python OCR implementation, configuration, Dockerfile, Compose service and native-model build/download lifecycle were removed. API configuration has one [llm] endpoint/model/key and extraction generation settings; no credentials enter clients or images.

Desk's local_llm config identified qwen3.8-flash-next-iq3_s. It owns model loading/unloading and GPU use. Its authenticated API is reachable by NAS on the LAN; existing loopback clients remain supported. On the user's instruction, local_llm was stopped during resource-intensive compilation and restarted for inference tests. The observed Java processes belonged to the temporary Android toolchain container, not a production service; that container exited after its successful build and the private builder was removed.

The shared API inference semaphore serializes OCR and Analytics without holding the normal read/write admission slots. Image decoding, scaling and response processing use the existing background CPU executor. Analytics now uses resolved analyte identities so confirmed OCR results with preserved null original metric IDs can participate without rewriting original evidence.

Validation completed:

- Rust formatting, Clippy, 7 library tests and 236 integration tests passed; 2 existing tests remained ignored.
- 14 development tests and the 76-route mobile API caller inventory passed.
- The Docker Android build passed unit/lint/package checks and verified all 851 native app methods; unchanged mobile source was packaged in the API image.
- A marked isolated API on NAS imported a synthetic two-marker image through the real shared model. Both dates were correct, results remained pending until review, original replies were archived, and confirmation triggered a real validated Analytics result.
- Production NAS runs longlivehealth-backend-api:unified-strata-20261105, with its existing data mount and Caddy address. Server status reports OCR and Analytics available; probe-providers succeeded from the NAS network using the production configuration.
- The former Desk OCR container and isolated test containers were removed. Existing production reports were not rescanned and their original data was retained.

## Existing Function Health PDF audit

The existing 11-page report was copied into a separate NAS API container and Docker
volume. Production storage was mounted read-only only for copying the PDF and computing
before/after observation fingerprints. Neither run used a production rescan or review
endpoint. All test observations stayed pending, and Analytics was disabled in the isolated
configuration.

The first real-model run retained all 126 printed rows and their collection dates, but
placed units (and, on one page, range labels) inside `raw_result`. This prevented downstream
numeric interpretation. Prompt `laboratory-page-v6` separates result, unit, report flag and
calculation notes while retaining the complete printed quote. `lab-units-v8` also derives
numbers from an appended unit only when its canonical identity matches the separately
supplied unit; ambiguous text and unit mismatches remain unparsed, and comparisons never
become exact points. This derivation does not rewrite stored observations or raw outputs.

The second full run used the rebuilt production API image and shared Strata model. Every
printed row was compared with the PDF page and extracted text: name, collection date,
result, unit, and downstream converted value/unit. All 126 rows passed: 103 exact numeric
results, 8 bounded comparisons, and 15 qualitative results. All collection dates were
present; no row was missing or added. Count multipliers and calculated-result annotations
were checked separately. One page retained a text-reading-order warning; visual comparison
confirmed its row associations. Repeated rows in the original were included in the audit.

Validation: Docker Rust formatting and Clippy passed; 7 library and 237 integration tests
passed, with 2 pre-existing ignored tests. The backend-only image keeps the existing Android
APK unchanged. NAS now runs `longlivehealth-backend-api:ocr-audit-20261105`. Fingerprints of
all 126 confirmed production observations (IDs, revisions, statuses, and full payloads) were
identical before and after testing and deployment. The real PDF, model replies and detailed
item audit are private test artifacts, not repository fixtures.

After the isolated audit, the user separately authorized correction of confirmed data.
Eight existing rows had calculation annotations in value/unit fields, and a subsequent
review of the remaining rows found one value with a repeated unit. These nine rows were
corrected from the verified model output in revision-checked transactions. Confirmation
status, collection dates, metric mappings and original evidence were preserved; unrelated
rows were unchanged. Report/user revisions and derived caches were updated. Final comparison
of all 126 confirmed rows with the audited PDF found no remaining field differences.

A repeat full-PDF run also verified that all 126 observations in the model's original reply
were identical to the OCR response observations, without field rewriting. Field separation
is instructed by the prompt; no keyword-specific OCR rejection or cleanup layer is included.
The existing structural, required-field and date-validity checks remain in place. These
checks do not assert that every future model response is semantically correct.

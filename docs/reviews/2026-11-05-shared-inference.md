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

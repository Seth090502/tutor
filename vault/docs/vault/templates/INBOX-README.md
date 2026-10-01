# Inbox

This is the only folder in the vault you write to. Full instructions: `..\INSTRUCTIONS.md` (sections 3 and 4).

- **Batch:** make `inbox\<date-name>\`, copy files in (add `manifest.tsv` if you have one), then create an empty
  `_BATCH_COMPLETE` file.
- **oa-forge harvest:** drop the whole root folder (the one with `_meta\manifest.jsonl`) here.
- **Single file:** put it in `loose\<category>\`.
- **Source code:** copy a repository (or a folder of repositories) into a batch folder. Remove secrets first.
- **After intake:** finished batches move to `_done\<batch>\` with `_INTAKE_REPORT.md`. What is left there is
  what the vault did not take. Delete it when you are satisfied.
- Names starting with `_` at this level belong to intake. Put copies here, never your only copy.

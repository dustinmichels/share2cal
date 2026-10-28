<script setup lang="ts">
const rows = [
  {
    q: 'Is mode "simple"?',
    branch: "yes",
    result: "Rule-based parser only",
    detail: "parse_events_deterministic",
    kind: "rules",
  },
  {
    q: "Does the schedule-table parser find 2 or more events?",
    branch: "yes",
    result: "Return the schedule events",
    detail: "parse_schedule_table_events",
    kind: "rules",
  },
  {
    q: "Is the model in the manifest and downloaded to disk?",
    branch: "no",
    result: "Fall back to rules",
    detail: 'source = "deterministic_fallback"',
    kind: "fallback",
  },
  {
    q: "Did the weights load (or are they already cached)?",
    branch: "no",
    result: "Fall back to rules",
    detail: "get_or_load_model",
    kind: "fallback",
  },
  {
    q: "Did generation finish before the timeout?",
    branch: "no",
    result: "Fall back to rules",
    detail: "default 5 s, run_inference_async",
    kind: "fallback",
  },
  {
    q: "Does the output parse into at least one event?",
    branch: "no",
    result: "Fall back to rules",
    detail: "parse_llm_json_payload",
    kind: "fallback",
  },
];
</script>

<template>
  <figure class="s2c-figure flow">
    <ol class="rows">
      <li v-for="(r, i) in rows" :key="i" class="row">
        <div class="q">{{ r.q }}</div>
        <div class="edge">
          <span>{{ r.branch }}</span>
        </div>
        <div class="result" :class="r.kind">
          <strong>{{ r.result }}</strong>
          <code>{{ r.detail }}</code>
        </div>
      </li>
      <li class="row last">
        <div class="q done">LLM events returned</div>
        <div class="edge none"></div>
        <div class="result llm">
          <strong>source = "llm", confidence 0.95</strong>
          <code>extract_events_orchestrated</code>
        </div>
      </li>
    </ol>
    <figcaption>
      Decision order inside <code>parse_events_internal</code> and
      <code>extract_events_orchestrated</code>. Every failure path ends in the same rule-based
      parser, so a parse call never errors because of the model.
    </figcaption>
  </figure>
</template>

<style scoped>
.flow p {
  margin: 0;
}

.rows {
  list-style: none;
  margin: 0 !important;
  padding: 0 !important;
  position: relative;
}

.rows::before {
  content: "";
  position: absolute;
  left: 18px;
  top: 20px;
  bottom: 20px;
  width: 2px;
  background: var(--vp-c-border);
}

.row {
  position: relative;
  display: grid;
  grid-template-columns: minmax(0, 1.1fr) 56px minmax(0, 1fr);
  align-items: center;
  margin: 0 0 12px !important;
  line-height: 1.4 !important;
}

.q {
  position: relative;
  margin-left: 0;
  padding: 10px 14px;
  background: var(--vp-c-bg-elv);
  border: 1.5px solid var(--vp-c-text-1);
  border-radius: 8px;
  font-size: 0.9rem;
  font-weight: 550;
}

.q.done {
  border-color: var(--s2c-blue);
  color: var(--s2c-blue);
}

.edge {
  position: relative;
  height: 2px;
  background: var(--vp-c-border);
}

.edge.none {
  background: none;
}

.edge span {
  position: absolute;
  left: 50%;
  top: -10px;
  transform: translateX(-50%);
  padding: 0 4px;
  background: var(--vp-c-bg);
  font-size: 0.72rem;
  font-weight: 650;
  color: var(--vp-c-text-2);
}

.result {
  display: grid;
  gap: 2px;
  padding: 8px 12px;
  border-radius: 8px;
  font-size: 0.86rem;
  border: 1px solid var(--vp-c-divider);
}

.result code {
  font-size: 0.74rem;
  background: none;
  padding: 0;
  overflow-wrap: anywhere;
  color: var(--vp-c-text-2);
}

.result.rules {
  background: var(--vp-c-brand-soft);
  border-color: transparent;
}

.result.fallback {
  border-style: dashed;
}

.result.llm {
  background: var(--s2c-pink-wash);
  border-color: var(--s2c-pink);
}

@media (max-width: 640px) {
  .rows::before {
    display: none;
  }

  .row {
    grid-template-columns: 1fr;
    gap: 6px;
    margin-bottom: 18px !important;
  }

  .edge {
    height: 14px;
    width: 2px;
    margin-left: 18px;
  }

  .edge span {
    left: 10px;
    top: 0;
    transform: none;
  }
}
</style>

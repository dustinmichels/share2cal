<script setup lang="ts">
import { withBase } from "vitepress";

const entries = [
  {
    q: "New to the repo?",
    a: "Install the toolchain with mise, then run the desktop build with one command.",
    links: [
      { text: "Getting started", href: "/guide/getting-started" },
      { text: "Project structure", href: "/guide/project-structure" },
    ],
  },
  {
    q: "Changing how events are extracted?",
    a: "Parsing is split between deterministic Rust rules and a prompt sent to a local model.",
    links: [
      { text: "Rule-based parsing", href: "/architecture/parsing" },
      { text: "On-device LLM", href: "/architecture/local-llm" },
    ],
  },
  {
    q: "Adding a native capability?",
    a: "Every feature is a Tauri command in Rust, often backed by an Objective-C bridge.",
    links: [
      { text: "Architecture", href: "/architecture/overview" },
      { text: "IPC commands", href: "/reference/commands" },
    ],
  },
];
</script>

<template>
  <section class="start">
    <h2 class="heading">Where to start</h2>
    <div class="grid">
      <div v-for="e in entries" :key="e.q" class="entry">
        <h3 class="q">{{ e.q }}</h3>
        <p class="a">{{ e.a }}</p>
        <p class="links">
          <a v-for="l in e.links" :key="l.href" :href="withBase(l.href)">{{ l.text }}</a>
        </p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.start {
  margin-top: 104px;
}

.heading {
  font-family: var(--s2c-display);
  font-weight: 750;
  font-stretch: 85%;
  font-size: clamp(1.9rem, 3.4vw, 2.6rem);
  letter-spacing: -0.025em;
  line-height: 1.05;
  margin: 0 0 36px;
  color: var(--s2c-ink);
}

.grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 40px;
}

.entry {
  padding-top: 20px;
  border-top: 2px solid var(--s2c-ink);
}

.q {
  margin: 0;
  font-family: var(--s2c-display);
  font-weight: 680;
  font-size: 1.25rem;
  line-height: 1.25;
  letter-spacing: -0.01em;
  color: var(--s2c-ink);
}

.a {
  margin: 10px 0 0;
  font-size: 0.97rem;
  line-height: 1.6;
  color: var(--s2c-ink-soft);
}

.links {
  display: flex;
  flex-wrap: wrap;
  gap: 18px;
  margin: 16px 0 0;
}

.links a {
  font-weight: 600;
  color: var(--s2c-blue);
  text-decoration: underline;
  text-underline-offset: 4px;
  text-decoration-thickness: 1.5px;
  text-decoration-color: color-mix(in srgb, var(--s2c-blue) 40%, transparent);
}

.links a:hover {
  text-decoration-color: currentColor;
}

@media (max-width: 860px) {
  .grid {
    grid-template-columns: 1fr;
    gap: 28px;
  }
}
</style>

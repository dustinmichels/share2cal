<script setup lang="ts">
import { withBase } from "vitepress";

const steps = [
  {
    title: "Share",
    body: "The iOS Share Extension writes the image into the App Group container and opens the app. On desktop you drag it in, paste it or pick a file.",
    files: ["ShareViewController.m", "share.rs"],
    link: "/architecture/sharing",
  },
  {
    title: "Read",
    body: "Apple Vision recognises each line of text with its bounding box and decodes any QR codes on the page.",
    files: ["ocr_apple.m", "ocr.rs"],
    link: "/architecture/ocr",
  },
  {
    title: "Parse",
    body: "A rule-based parser returns a first draft straight away. Class schedules and agendas become several events.",
    files: ["parser/"],
    link: "/architecture/parsing",
  },
  {
    title: "Refine",
    body: "In Enhanced mode a small GGUF model re-reads the text and replaces the draft. If it fails or times out, the rules win.",
    files: ["inference.rs", "model.rs"],
    link: "/architecture/local-llm",
  },
  {
    title: "Save",
    body: "You review and edit, then write to the device calendar through EventKit, export an .ics file or open Google Calendar.",
    files: ["calendar.rs", "calendar_apple.m"],
    link: "/architecture/calendar",
  },
];
</script>

<template>
  <section class="pipeline">
    <h2 class="heading">What happens to a shared image</h2>
    <ol class="track">
      <li v-for="(step, i) in steps" :key="step.title" class="step">
        <span class="num">{{ i + 1 }}</span>
        <h3 class="title">
          <a :href="withBase(step.link)">{{ step.title }}</a>
        </h3>
        <p class="body">{{ step.body }}</p>
        <p class="files">
          <code v-for="f in step.files" :key="f">{{ f }}</code>
        </p>
      </li>
    </ol>
  </section>
</template>

<style scoped>
.pipeline {
  padding-top: 24px;
}

.heading {
  font-family: var(--s2c-display);
  font-weight: 750;
  font-stretch: 85%;
  font-size: clamp(1.9rem, 3.4vw, 2.6rem);
  letter-spacing: -0.025em;
  line-height: 1.05;
  margin: 0 0 40px;
  color: var(--s2c-ink);
}

.track {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 28px;
  position: relative;
}

/* The connecting rule runs through the step numbers */
.track::before {
  content: "";
  position: absolute;
  top: 17px;
  left: 18px;
  right: 18px;
  height: 2px;
  background: repeating-linear-gradient(to right, var(--vp-c-border) 0 8px, transparent 8px 14px);
}

.step {
  position: relative;
}

.num {
  position: relative;
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: var(--s2c-paper);
  border: 2px solid var(--s2c-ink);
  font-family: var(--s2c-display);
  font-weight: 700;
  font-size: 15px;
  color: var(--s2c-ink);
}

.step:nth-child(4) .num {
  border-color: var(--s2c-pink);
  color: var(--s2c-pink);
}

.title {
  margin: 18px 0 8px;
  font-family: var(--s2c-display);
  font-weight: 700;
  font-size: 1.3rem;
  letter-spacing: -0.01em;
}

.title a {
  color: var(--s2c-ink);
  text-decoration: none;
}

.title a:hover {
  color: var(--s2c-blue);
}

.body {
  margin: 0;
  font-size: 0.95rem;
  line-height: 1.6;
  color: var(--s2c-ink-soft);
}

.files {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin: 14px 0 0;
}

.files code {
  font-family: var(--vp-font-family-mono);
  font-size: 0.75rem;
  padding: 2px 7px;
  border-radius: 4px;
  background: var(--vp-code-bg);
  color: var(--vp-code-color);
}

@media (max-width: 960px) {
  .track {
    grid-template-columns: 1fr;
    gap: 32px;
    padding-left: 56px;
  }

  .track::before {
    top: 18px;
    bottom: 18px;
    left: 17px;
    right: auto;
    width: 2px;
    height: auto;
    background: repeating-linear-gradient(
      to bottom,
      var(--vp-c-border) 0 8px,
      transparent 8px 14px
    );
  }

  .num {
    position: absolute;
    left: -56px;
    top: -4px;
  }

  .title {
    margin-top: 0;
  }
}
</style>

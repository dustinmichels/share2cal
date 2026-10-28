<script setup lang="ts">
import { withBase } from "vitepress";
</script>

<template>
  <section class="hero">
    <div class="copy">
      <h1 class="headline">Flyer in, calendar event out.</h1>
      <p class="lede">
        Share2Cal takes a screenshot or a photo of a poster, reads the text, works out the title,
        time and place, and adds the event to your calendar. Text recognition, parsing and the
        language model all run on the phone.
      </p>
      <div class="actions">
        <a class="btn primary" :href="withBase('/guide/getting-started')">Set up the project</a>
        <a class="btn ghost" :href="withBase('/architecture/overview')">See how it works</a>
      </div>
      <p class="stack">Tauri 2, Vue 3, Rust, llama.cpp, Apple Vision and EventKit.</p>
    </div>

    <div class="stage" aria-hidden="true">
      <div class="flyer">
        <span class="tape"></span>
        <div class="scan"></div>

        <p class="f-kicker"><span class="box b1"></span>Open studio, all levels</p>
        <p class="f-title"><span class="box b2"></span>Pottery<br />Night</p>
        <p class="f-when"><span class="box b3"></span>Thursday, Nov 12<br />6 – 8 PM</p>
        <p class="f-where"><span class="box b4"></span>Gilman Street Studio</p>
        <div class="f-foot">
          <svg class="qr" viewBox="0 0 21 21" shape-rendering="crispEdges">
            <path
              fill="currentColor"
              d="M0 0h7v7H0zM1 1v5h5V1zM2 2h3v3H2zM14 0h7v7h-7zM15 1v5h5V1zM16 2h3v3h-3zM0 14h7v7H0zM1 15v5h5v-5zM2 16h3v3H2zM8 0h2v2H8zM11 1h2v3h-2zM8 3h2v2H8zM9 6h3v2H9zM8 9h2v3H8zM11 9h3v2h-3zM15 8h2v2h-2zM18 9h3v2h-3zM0 8h2v2H0zM3 9h3v2H3zM12 12h3v2h-3zM16 12h2v3h-2zM19 13h2v2h-2zM8 14h2v2H8zM11 15h2v3h-2zM14 16h2v2h-2zM8 18h3v3H8zM17 17h4v2h-4zM14 19h2v2h-2zM19 20h2v1h-2z"
            />
          </svg>
          <span class="box b5"></span>
        </div>
      </div>

      <div class="event">
        <div class="ev-bar"></div>
        <div class="ev-body">
          <p class="ev-title">Pottery Night</p>
          <p class="ev-line">Thu, Nov 12, 6:00 – 8:00 PM</p>
          <p class="ev-line">Gilman Street Studio</p>
          <p class="ev-added">
            <svg viewBox="0 0 16 16" width="14" height="14">
              <path
                d="M3 8.5l3 3 7-7"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
            Added to Calendar
          </p>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.hero {
  display: grid;
  grid-template-columns: minmax(0, 1.05fr) minmax(0, 1fr);
  gap: 56px;
  align-items: center;
  padding: 72px 0 88px;
}

.headline {
  font-family: var(--s2c-display);
  font-weight: 800;
  font-stretch: 78%;
  font-size: clamp(3rem, 6.6vw, 5.4rem);
  line-height: 0.94;
  letter-spacing: -0.035em;
  color: var(--s2c-ink);
  margin: 0;
  text-wrap: balance;
}

.lede {
  margin: 28px 0 0;
  max-width: 46ch;
  font-size: 1.15rem;
  line-height: 1.6;
  color: var(--s2c-ink-soft);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: 32px;
}

.btn {
  display: inline-flex;
  align-items: center;
  height: 46px;
  padding: 0 22px;
  border-radius: 999px;
  font-weight: 600;
  font-size: 0.98rem;
  text-decoration: none;
  transition:
    background-color 0.15s,
    border-color 0.15s,
    color 0.15s;
}

.btn.primary {
  background: var(--s2c-blue);
  color: #fff;
}

.dark .btn.primary {
  color: #0d1424;
}

.btn.primary:hover {
  background: var(--s2c-blue-deep);
}

.btn.ghost {
  border: 1.5px solid var(--vp-c-border);
  color: var(--s2c-ink);
}

.btn.ghost:hover {
  border-color: var(--s2c-ink);
}

.stack {
  margin: 28px 0 0;
  font-size: 0.9rem;
  color: var(--s2c-ink-faint);
}

/* ---------- Stage ---------- */

.stage {
  position: relative;
  min-height: 470px;
}

.flyer {
  position: absolute;
  top: 0;
  left: 4%;
  width: 290px;
  padding: 30px 28px 24px;
  background: #fffdf8;
  color: #1b1b2b;
  border-radius: 3px;
  transform: rotate(-4deg);
  box-shadow:
    0 1px 0 rgba(19, 32, 58, 0.08),
    0 18px 40px -18px rgba(19, 32, 58, 0.45);
  overflow: hidden;
}

.dark .flyer {
  background: #f1eee6;
}

.tape {
  position: absolute;
  top: -8px;
  left: 50%;
  width: 92px;
  height: 26px;
  transform: translateX(-50%) rotate(3deg);
  background: rgba(10, 108, 196, 0.22);
  backdrop-filter: blur(1px);
}

.flyer p {
  position: relative;
  margin: 0;
}

.box {
  position: absolute;
  inset: -4px -6px;
  border: 1.5px solid var(--s2c-pink);
  background: rgba(230, 46, 140, 0.07);
  border-radius: 2px;
  opacity: 0;
  animation: box-in 0.28s ease-out both;
}

.f-kicker {
  font-size: 12px;
  font-weight: 600;
  color: #6a5f8f;
}

.f-title {
  margin-top: 14px !important;
  font-family: var(--s2c-display);
  font-weight: 800;
  font-stretch: 75%;
  font-size: 66px;
  line-height: 0.86;
  letter-spacing: -0.03em;
  color: #e0357d;
}

.f-when {
  margin-top: 22px !important;
  font-family: var(--s2c-display);
  font-weight: 650;
  font-size: 20px;
  line-height: 1.15;
}

.f-where {
  margin-top: 14px !important;
  font-size: 13px;
  font-weight: 500;
  color: #3d3d55;
}

.f-foot {
  position: relative;
  width: 44px;
  margin-top: 22px;
  color: #1b1b2b;
}

.qr {
  display: block;
  width: 44px;
  height: 44px;
}

.b1 {
  animation-delay: 0.45s;
}
.b2 {
  animation-delay: 0.7s;
}
.b3 {
  animation-delay: 1.05s;
}
.b4 {
  animation-delay: 1.25s;
}
.b5 {
  animation-delay: 1.45s;
}

.scan {
  position: absolute;
  left: 0;
  right: 0;
  top: 0;
  height: 3px;
  background: var(--s2c-pink);
  box-shadow: 0 0 18px 4px rgba(230, 46, 140, 0.45);
  opacity: 0;
  animation: scan 1.5s cubic-bezier(0.45, 0, 0.4, 1) 0.3s both;
  z-index: 2;
}

.event {
  position: absolute;
  right: 0;
  bottom: 8px;
  display: flex;
  width: 300px;
  background: var(--s2c-paper-raised);
  border: 1px solid var(--vp-c-border);
  border-radius: 14px;
  box-shadow: 0 24px 50px -24px rgba(19, 32, 58, 0.5);
  overflow: hidden;
  animation: event-in 0.55s cubic-bezier(0.2, 0.8, 0.2, 1) 1.8s both;
}

.ev-bar {
  width: 6px;
  flex: none;
  background: var(--s2c-blue);
}

.ev-body {
  padding: 18px 20px 16px;
}

.ev-body p {
  margin: 0;
}

.ev-title {
  font-family: var(--s2c-display);
  font-weight: 700;
  font-size: 1.35rem;
  letter-spacing: -0.01em;
  color: var(--s2c-ink);
}

.ev-line {
  margin-top: 4px !important;
  font-size: 0.93rem;
  color: var(--s2c-ink-soft);
}

.ev-added {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 14px !important;
  padding-top: 12px;
  border-top: 1px solid var(--vp-c-divider);
  font-size: 0.86rem;
  font-weight: 600;
  color: #1f9d55;
  animation: fade 0.3s ease-out 2.35s both;
}

.dark .ev-added {
  color: #4ade80;
}

@keyframes scan {
  0% {
    transform: translateY(0);
    opacity: 1;
  }
  90% {
    opacity: 1;
  }
  100% {
    transform: translateY(470px);
    opacity: 0;
  }
}

@keyframes box-in {
  from {
    opacity: 0;
    transform: scale(1.06);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

@keyframes event-in {
  from {
    opacity: 0;
    transform: translate(-28px, -18px) rotate(-2deg);
  }
  to {
    opacity: 1;
    transform: none;
  }
}

@keyframes fade {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@media (max-width: 960px) {
  .hero {
    grid-template-columns: 1fr;
    gap: 40px;
    padding: 48px 0 64px;
  }

  .stage {
    max-width: 520px;
  }
}

@media (max-width: 480px) {
  .stage {
    min-height: 520px;
  }

  .flyer {
    left: 0;
    width: 250px;
  }

  .f-title {
    font-size: 56px;
  }

  .event {
    width: 260px;
  }
}
</style>

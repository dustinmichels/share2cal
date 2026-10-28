<template>
  <figure class="s2c-figure arch">
    <div class="ext">
      <div class="ext-box">
        <p class="layer-name">Share Extension</p>
        <p class="note">Separate iOS process</p>
        <code>ShareViewController.m</code>
      </div>
      <div class="ext-arrow">
        <span
          >writes image + <code>pending_share.json</code>, then opens
          <code>share2cal://share</code></span
        >
      </div>
    </div>

    <div class="app">
      <p class="app-label">Share2Cal app process</p>

      <div class="layer web">
        <p class="layer-name">WebView: Vue 3 frontend</p>
        <div class="chips">
          <code>App.vue</code><code>components/*.vue</code><code>services/*.ts</code>
        </div>
      </div>

      <div class="bridge">
        <span class="down">invoke("command", args)</span>
        <span class="up">emit("model_download_progress")</span>
      </div>

      <div class="layer rust">
        <p class="layer-name">Rust core: <code>share2cal_lib</code></p>
        <div class="chips">
          <code class="strong">commands/</code>
          <code>share</code><code>ocr</code><code>parser</code><code>inference</code
          ><code>model</code><code>calendar</code>
        </div>
      </div>

      <div class="bridge thin">
        <span class="down">C FFI, Rust bindings</span>
      </div>

      <div class="layer native">
        <div class="native-col">
          <p class="layer-name">Objective-C bridges</p>
          <div class="chips">
            <code>share_apple.m</code><code>ocr_apple.m</code><code>calendar_apple.m</code>
          </div>
        </div>
        <div class="native-col">
          <p class="layer-name">llama.cpp</p>
          <div class="chips"><code>llama-cpp-2</code><code>Metal</code></div>
        </div>
      </div>
    </div>

    <div class="os">
      <span>App Group container</span>
      <span>Vision</span>
      <span>EventKit</span>
      <span>Metal GPU</span>
      <span class="net">Hugging Face (model downloads)</span>
    </div>

    <figcaption>
      Everything except the Share Extension runs in one process. The Vue app never touches the OS
      directly; it calls Rust commands, and Rust reaches Apple frameworks through small Objective-C
      files compiled by <code>build.rs</code>.
    </figcaption>
  </figure>
</template>

<style scoped>
.arch {
  font-size: 0.88rem;
}

.arch p {
  margin: 0;
}

.arch code {
  font-size: 0.8em;
}

.layer-name {
  font-family: var(--s2c-display);
  font-weight: 650;
  font-size: 0.98rem;
  color: var(--vp-c-text-1);
}

.note {
  font-size: 0.8rem;
  color: var(--vp-c-text-3);
}

.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}

.chips code {
  background: var(--vp-c-bg);
  border: 1px solid var(--vp-c-divider);
}

.chips code.strong {
  border-color: var(--vp-c-brand-1);
  color: var(--vp-c-brand-1);
}

/* Share extension row */
.ext {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 16px;
  align-items: center;
  margin-bottom: 14px;
}

.ext-box {
  padding: 12px 16px;
  border: 1.5px dashed var(--s2c-pink);
  border-radius: 10px;
  display: grid;
  gap: 4px;
}

.ext-arrow {
  position: relative;
  padding-left: 28px;
  font-size: 0.82rem;
  color: var(--vp-c-text-2);
}

.ext-arrow::before {
  content: "";
  position: absolute;
  left: 0;
  top: 50%;
  width: 20px;
  height: 2px;
  background: var(--s2c-pink);
}

/* App container */
.app {
  position: relative;
  padding: 30px 16px 16px;
  border: 2px solid var(--vp-c-text-1);
  border-radius: 14px;
}

.app-label {
  position: absolute;
  top: -11px;
  left: 16px;
  padding: 0 8px;
  background: var(--vp-c-bg);
  font-family: var(--s2c-display);
  font-weight: 700;
  font-size: 0.9rem;
}

.layer {
  padding: 12px 14px;
  border-radius: 8px;
  background: var(--vp-c-bg-soft);
}

.layer.web {
  box-shadow: inset 3px 0 0 var(--s2c-indigo);
}

.layer.rust {
  box-shadow: inset 3px 0 0 var(--s2c-blue);
}

.layer.native {
  display: grid;
  grid-template-columns: 1.4fr 1fr;
  gap: 16px;
  box-shadow: inset 3px 0 0 var(--vp-c-text-3);
}

.bridge {
  display: flex;
  justify-content: center;
  gap: 28px;
  padding: 8px 0;
  font-family: var(--vp-font-family-mono);
  font-size: 0.74rem;
  color: var(--vp-c-text-2);
}

.bridge span::before {
  margin-right: 6px;
  color: var(--vp-c-brand-1);
}

.bridge .down::before {
  content: "↓";
}

.bridge .up::before {
  content: "↑";
}

.os {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 14px;
}

.os span {
  padding: 6px 12px;
  border-radius: 999px;
  border: 1px solid var(--vp-c-border);
  font-size: 0.8rem;
  color: var(--vp-c-text-2);
}

.os .net {
  border-style: dashed;
}

@media (max-width: 640px) {
  .ext {
    grid-template-columns: 1fr;
  }

  .ext-arrow {
    padding-left: 0;
    padding-top: 6px;
  }

  .ext-arrow::before {
    display: none;
  }

  .layer.native {
    grid-template-columns: 1fr;
  }

  .bridge {
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }
}
</style>

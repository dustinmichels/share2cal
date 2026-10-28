<script setup lang="ts">
defineProps<{
  items: { who: string; what: string; code?: string }[];
}>();

const lanes: Record<string, string> = {
  Extension: "pink",
  Frontend: "indigo",
  Rust: "blue",
  Native: "grey",
  You: "ink",
};
</script>

<template>
  <ol class="timeline">
    <li v-for="(item, i) in items" :key="i" :class="['item', lanes[item.who] ?? 'grey']">
      <span class="who">{{ item.who }}</span>
      <div class="what">
        <p v-html="item.what"></p>
        <code v-if="item.code">{{ item.code }}</code>
      </div>
    </li>
  </ol>
</template>

<style scoped>
.timeline {
  list-style: none;
  margin: 24px 0 32px !important;
  padding: 0 !important;
  counter-reset: step;
}

.item {
  counter-increment: step;
  display: grid;
  grid-template-columns: 96px minmax(0, 1fr);
  gap: 40px;
  position: relative;
  margin: 0 !important;
  padding: 0 0 18px;
}

.item::before {
  content: "";
  position: absolute;
  left: 114px;
  top: 10px;
  bottom: -2px;
  width: 2px;
  background: var(--vp-c-divider);
}

.item:last-child::before {
  display: none;
}

.who {
  justify-self: end;
  align-self: start;
  margin-top: 2px;
  padding: 1px 10px;
  border-radius: 999px;
  font-size: 0.76rem;
  font-weight: 650;
  line-height: 1.6;
  color: var(--lane);
  border: 1.5px solid var(--lane);
}

.what {
  position: relative;
  padding-left: 0;
}

.what::before {
  content: counter(step);
  position: absolute;
  left: -31px;
  top: 3px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  background: var(--vp-c-bg);
  border: 2px solid var(--lane);
  font-size: 0.66rem;
  font-weight: 700;
  color: var(--lane);
}

.what p {
  margin: 0 !important;
  line-height: 1.6 !important;
}

.what code {
  display: inline-block;
  margin-top: 4px;
  font-size: 0.78em;
}

.pink {
  --lane: var(--s2c-pink);
}
.indigo {
  --lane: var(--s2c-indigo);
}
.blue {
  --lane: var(--s2c-blue);
}
.grey {
  --lane: var(--vp-c-text-2);
}
.ink {
  --lane: var(--vp-c-text-1);
}

@media (max-width: 560px) {
  .item {
    grid-template-columns: 1fr;
    gap: 6px;
    padding-left: 28px;
  }

  .item::before {
    left: 9px;
    top: 28px;
  }

  .who {
    justify-self: start;
  }

  .what {
    padding-left: 0;
  }

  .what::before {
    left: -28px;
    top: -26px;
  }
}
</style>

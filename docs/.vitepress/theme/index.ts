import DefaultTheme from "vitepress/theme";
import type { Theme } from "vitepress";

import "@fontsource-variable/bricolage-grotesque/standard.css";
import "@fontsource-variable/instrument-sans";
import "@fontsource-variable/instrument-sans/wght-italic.css";
import "@fontsource-variable/jetbrains-mono";
import "./custom.css";

import HomeHero from "./components/HomeHero.vue";
import HomePipeline from "./components/HomePipeline.vue";
import HomeStart from "./components/HomeStart.vue";
import ArchDiagram from "./components/ArchDiagram.vue";
import ParseFlow from "./components/ParseFlow.vue";
import Timeline from "./components/Timeline.vue";

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component("HomeHero", HomeHero);
    app.component("HomePipeline", HomePipeline);
    app.component("HomeStart", HomeStart);
    app.component("ArchDiagram", ArchDiagram);
    app.component("ParseFlow", ParseFlow);
    app.component("Timeline", Timeline);
  },
} satisfies Theme;

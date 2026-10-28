import { defineConfig } from "vitepress";

export default defineConfig({
  title: "Share2Cal",
  description: "How Share2Cal turns a shared flyer or screenshot into calendar events, on-device.",
  lang: "en-US",
  cleanUrls: true,
  lastUpdated: true,
  head: [
    ["link", { rel: "icon", type: "image/svg+xml", href: "/icon.svg" }],
    ["meta", { name: "theme-color", content: "#0a6cc4" }],
  ],
  markdown: {
    theme: { light: "github-light", dark: "github-dark-dimmed" },
  },
  themeConfig: {
    logo: { src: "/icon.svg", alt: "" },
    nav: [
      { text: "Guide", link: "/guide/introduction", activeMatch: "/guide/" },
      { text: "How it works", link: "/architecture/overview", activeMatch: "/architecture/" },
      { text: "Reference", link: "/reference/commands", activeMatch: "/reference/" },
    ],
    sidebar: [
      {
        text: "Guide",
        items: [
          { text: "Introduction", link: "/guide/introduction" },
          { text: "Getting started", link: "/guide/getting-started" },
          { text: "Running on iPhone", link: "/guide/iphone" },
          { text: "Project structure", link: "/guide/project-structure" },
        ],
      },
      {
        text: "How it works",
        items: [
          { text: "Architecture", link: "/architecture/overview" },
          { text: "The scan pipeline", link: "/architecture/pipeline" },
          { text: "Getting images in", link: "/architecture/sharing" },
          { text: "Reading text (OCR)", link: "/architecture/ocr" },
          { text: "Rule-based parsing", link: "/architecture/parsing" },
          { text: "On-device LLM", link: "/architecture/local-llm" },
          { text: "Saving to a calendar", link: "/architecture/calendar" },
          { text: "Frontend", link: "/architecture/frontend" },
        ],
      },
      {
        text: "Reference",
        items: [
          { text: "IPC commands", link: "/reference/commands" },
          { text: "Data types", link: "/reference/data-model" },
          { text: "Configuration", link: "/reference/configuration" },
          { text: "Testing", link: "/reference/testing" },
        ],
      },
    ],
    socialLinks: [{ icon: "github", link: "https://github.com/dustinmichels/share2cal" }],
    editLink: {
      pattern: "https://github.com/dustinmichels/share2cal/edit/main/docs/:path",
      text: "Edit this page on GitHub",
    },
    outline: { level: [2, 3] },
    search: { provider: "local" },
    footer: {
      message: "Share2Cal developer documentation",
    },
  },
});

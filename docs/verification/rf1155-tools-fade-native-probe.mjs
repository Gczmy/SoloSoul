import { readFileSync, writeFileSync } from "node:fs";
function probe() {
  const frame = () => new Promise(requestAnimationFrame);
  const wait = async (condition) => {
    for (let i = 0; i < 150; i++) {
      if (condition()) return;
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
    throw new Error("Public Tools probe timed out");
  };
  const report = (report) =>
    window.__TAURI_INTERNALS__.invoke("frame_frontend_report", { report });
  (async () => {
    await wait(
      () =>
        document.querySelector("[data-sidebar-tools]") &&
        document.documentElement.dataset.windowsFrame === "true",
    );
    await new Promise((resolve) => setTimeout(resolve, 750));
    const environment = {
      reduceMotion: document.documentElement.dataset.reduceMotion,
      mediaReduced: matchMedia("(prefers-reduced-motion: reduce)").matches,
      userAgent: navigator.userAgent,
    };
    const results = [];
    for (const position of ["left", "right"]) {
      if (position === "right") {
        Array.from(document.querySelectorAll("[data-appbar] button"))
          .find((button) => button.textContent === "Right navigation")
          .click();
        await frame();
        await frame();
      }
      const nav = document.querySelector("#desktop-navigation");
      const menu = nav.querySelector("[data-sidebar-tools]");
      const zone = nav.querySelector('[class*="primaryZone"]');
      const wrapper = menu.parentElement;
      // Only this account-free fixture restores the source's normal transition
      // declaration. OS settings and production reduced-motion behavior stay intact.
      for (const node of [menu, zone]) {
        const candidates = Array.from(document.styleSheets)
          .flatMap((sheet) => Array.from(sheet.cssRules))
          .filter(
            (rule) =>
              rule instanceof CSSStyleRule &&
              rule.selectorText.includes("sideNav") &&
              !rule.selectorText.includes("reduce-motion") &&
              node.matches(rule.selectorText) &&
              rule.style.transition.includes("clip-path"),
          );
        if (candidates.length !== 1)
          throw new Error("Ambiguous production transition rule");
        node.style.setProperty(
          "transition",
          candidates[0].style.transition,
          "important",
        );
      }
      await frame();
      await frame();
      const closedDelay = getComputedStyle(menu).transitionDelay;
      const openRule = Array.from(document.styleSheets)
        .flatMap((sheet) => Array.from(sheet.cssRules))
        .find(
          (rule) =>
            rule instanceof CSSStyleRule &&
            rule.selectorText.includes("foldableAreaOpen") &&
            rule.style.transitionDelay,
        );
      if (!openRule)
        throw new Error("Production open transition delay missing");
      const sweep = async (opening) => {
        menu.style.setProperty(
          "transition-delay",
          opening ? openRule.style.transitionDelay : closedDelay,
          "important",
        );
        let captured = false;
        const all = [];
        const capture = () => {
          const animations = [
            ...menu.getAnimations(),
            ...zone.getAnimations(),
          ].filter((a) => a instanceof CSSTransition);
          if (
            !animations.some((a) => a.transitionProperty === "opacity") ||
            !menu
              .getAnimations()
              .some(
                (a) =>
                  a instanceof CSSTransition &&
                  a.transitionProperty === "clip-path",
              ) ||
            !zone
              .getAnimations()
              .some(
                (a) =>
                  a instanceof CSSTransition &&
                  a.transitionProperty === "clip-path",
              )
          )
            return;
          for (const a of animations) {
            a.pause();
            all.push(a);
          }
          captured = true;
          nav.removeEventListener("transitionrun", capture);
        };
        nav.addEventListener("transitionrun", capture);
        wrapper.dispatchEvent(
          new MouseEvent(opening ? "mouseenter" : "mouseleave", {
            relatedTarget: document.body,
          }),
        );
        await wait(() => captured);
        const reveal = menu
          .getAnimations()
          .find(
            (a) =>
              a instanceof CSSTransition &&
              a.transitionProperty === "clip-path",
          );
        const opacity = menu
          .getAnimations()
          .find(
            (a) =>
              a instanceof CSSTransition && a.transitionProperty === "opacity",
          );
        const duration = Number(reveal.effect.getTiming().duration);
        const fadeDuration = Number(opacity.effect.getTiming().duration);
        const samples = [];
        for (const p of [0.15, 0.5, 0.85]) {
          for (const a of all) a.currentTime = duration * p;
          await frame();
          const style = getComputedStyle(menu);
          const top = parseFloat(style.clipPath.slice(6));
          samples.push({
            p,
            top,
            opacity: Number(style.opacity),
            visibility: style.visibility,
            synchronized:
              Math.abs(Number(style.opacity) - (1 - top / 100)) < 0.01,
          });
        }
        for (const a of all) a.finish();
        await frame();
        await frame();
        return {
          opening,
          duration,
          fadeDuration,
          inert: menu.inert,
          samples,
          pass:
            fadeDuration === duration &&
            samples.every(
              (s) =>
                s.synchronized &&
                s.opacity > 0 &&
                s.opacity < 1 &&
                s.visibility === "visible",
            ),
        };
      };
      for (const collapsed of [false, true]) {
        if ((nav.dataset.expanded === "false") !== collapsed) {
          nav
            .querySelector('button[aria-controls="desktop-navigation"]')
            .click();
          await frame();
          await frame();
        }
        results.push({
          position,
          collapsed,
          open: await sweep(true),
          close: await sweep(false),
        });
      }
      menu.style.removeProperty("transition");
      menu.style.removeProperty("transition-delay");
      zone.style.removeProperty("transition");
      document.documentElement.dataset.userReduceMotion = "true";
      await frame();
      wrapper.dispatchEvent(new MouseEvent("mouseenter"));
      await frame();
      await frame();
      const reducedOpen = {
        duration: getComputedStyle(menu).transitionDuration,
        opacity: getComputedStyle(menu).opacity,
        inert: menu.inert,
      };
      wrapper.dispatchEvent(
        new MouseEvent("mouseleave", { relatedTarget: document.body }),
      );
      await frame();
      await frame();
      const reducedClose = {
        opacity: getComputedStyle(menu).opacity,
        visibility: getComputedStyle(menu).visibility,
        inert: menu.inert,
      };
      await report({
        scope: "rf1155-public-native-reduced-motion",
        position,
        reducedOpen,
        reducedClose,
      });
      document.documentElement.dataset.userReduceMotion = "false";
    }
    await report({
      scope: "rf1155-public-native-tools-fade",
      environment,
      sourceNormalTransitionsRestoredInFixtureOnly: true,
      results,
      completed: true,
      pass: results.every((r) => r.open.pass && r.close.pass),
    });
  })().catch((error) =>
    report({ scope: "rf1155-public-native-tools-fade", error: String(error) }),
  );
}
const [input, output] = process.argv.slice(2);
if (!input || !output)
  throw new Error("Expected source fixture and new probe HTML");
writeFileSync(
  output,
  readFileSync(input, "utf8").replace(
    "</body>",
    `<script>(${probe.toString()})()</script></body>`,
  ),
);

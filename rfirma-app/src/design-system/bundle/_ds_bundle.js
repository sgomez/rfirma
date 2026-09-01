/* @ds-bundle: {"namespace":"RFirma","components":[],"sourceHashes":{},"inlinedExternals":[],"builtBy":"cc-design-sync"} */
(function(){
  // rFirma Design System - CSS-only system. There are no React components to
  // import: compose plain elements with the rf-* classes from _ds_bundle.css.
  // This global exposes the token values for code that needs them at runtime.
  var read = function(n){ try { return getComputedStyle(document.documentElement).getPropertyValue(n).trim(); } catch(e){ return ""; } };
  window.RFirma = {
    version: "alpha",
    kind: "css-tokens",
    token: read,
    tokens: {
      color: ["--rf-color-primary","--rf-color-on-primary","--rf-color-background","--rf-color-surface","--rf-color-border","--rf-color-text","--rf-color-text-muted","--rf-color-accent","--rf-text-on-dark","--rf-text-on-light","--rf-text-muted-on-dark","--rf-text-muted-on-light","--rf-border-subtle","--rf-border-strong"],
      space: ["--rf-space-1","--rf-space-2","--rf-space-3","--rf-space-4","--rf-space-5","--rf-space-6","--rf-space-7","--rf-space-8","--rf-space-9"],
      radius: ["--rf-radius-sm","--rf-radius-md","--rf-radius-lg","--rf-radius-xl","--rf-radius-pill"],
      shadow: ["--rf-shadow-card","--rf-shadow-elevated"],
      motion: ["--rf-duration-fast","--rf-duration-base","--rf-duration-slow","--rf-easing"],
      breakpoint: ["--rf-bp-xs","--rf-bp-sm","--rf-bp-md","--rf-bp-lg","--rf-bp-xl","--rf-bp-2xl"]
    }
  };
})();

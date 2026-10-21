(function () {
  "use strict";

  if ("serviceWorker" in navigator) {
    navigator.serviceWorker.getRegistrations().then(registrations => {
      for (const registration of registrations) {
        registration.update();
      }
    });
  }

  const OWNED_CACHE_PREFIXES = ["eltharn_gate", "ronner", "coi-serviceworker"];
  if ("caches" in window) {
    caches.keys().then(names => {
      for (const name of names) {
        if (OWNED_CACHE_PREFIXES.some(prefix => name.startsWith(prefix))) {
          caches.delete(name);
        }
      }
    });
  }
})();

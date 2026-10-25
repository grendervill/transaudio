(() => {
  const app = document.querySelector(".app");
  if (!app) return;

  const params = new URLSearchParams(window.location.search);
  const defaultScreen = app.dataset.defaultScreen || "transcribe";
  const screens = [...document.querySelectorAll("[data-screen]")];
  const navItems = [...document.querySelectorAll("[data-go]")];
  let toastTimer;

  function showToast(message) {
    let toast = document.querySelector(".toast");
    if (!toast) {
      toast = document.createElement("div");
      toast.className = "toast";
      toast.setAttribute("role", "status");
      toast.setAttribute("aria-live", "polite");
      document.body.appendChild(toast);
    }
    toast.textContent = message;
    requestAnimationFrame(() => toast.classList.add("is-visible"));
    clearTimeout(toastTimer);
    toastTimer = window.setTimeout(() => {
      toast.classList.remove("is-visible");
    }, 3600);
  }

  function goTo(name, updateUrl = true) {
    const target = screens.find((screen) => screen.dataset.screen === name);
    if (!target) return;
    app.dataset.current = name;
    screens.forEach((screen) => {
      screen.classList.toggle("is-active", screen === target);
      screen.setAttribute("aria-hidden", screen === target ? "false" : "true");
    });
    navItems.forEach((item) => {
      const active = item.dataset.go === name;
      item.classList.toggle("is-active", active);
      if (active) item.setAttribute("aria-current", "page");
      else item.removeAttribute("aria-current");
    });
    if (updateUrl) {
      const next = new URL(window.location.href);
      next.searchParams.set("screen", name);
      history.replaceState({}, "", next);
    }
    const scroller = target.querySelector("[data-screen-scroll]");
    if (scroller) scroller.scrollTop = 0;
  }

  navItems.forEach((item) => {
    item.addEventListener("click", (event) => {
      if (item.tagName === "A" && item.getAttribute("href") !== "#") return;
      event.preventDefault();
      goTo(item.dataset.go);
    });
  });

  const requestedScreen = params.get("screen");
  goTo(screens.some((screen) => screen.dataset.screen === requestedScreen) ? requestedScreen : defaultScreen, false);

  // Modals
  const backdrop = (name) => document.querySelector(`[data-modal="${name}"]`);
  let lastFocused = null;
  const focusableIn = (root) =>
    [...root.querySelectorAll("button, [href], input, select, textarea, [tabindex]:not([tabindex='-1'])")]
      .filter((element) => !element.disabled && element.getClientRects().length > 0);

  function openModal(name) {
    const modal = backdrop(name);
    if (!modal) return;
    lastFocused = document.activeElement;
    modal.classList.add("is-open");
    modal.removeAttribute("hidden");
    document.body.classList.add("has-modal");
    app.inert = true;
    const focusable = focusableIn(modal)[0];
    if (focusable) window.setTimeout(() => focusable.focus(), 50);
  }
  function closeModal(modal) {
    if (!modal) return;
    modal.classList.remove("is-open");
    modal.setAttribute("hidden", "");
    if (!document.querySelector(".modal-backdrop.is-open")) {
      document.body.classList.remove("has-modal");
      app.inert = false;
      if (lastFocused && typeof lastFocused.focus === "function") lastFocused.focus();
    }
  }

  document.querySelectorAll("[data-open-modal]").forEach((button) => {
    button.addEventListener("click", () => openModal(button.dataset.openModal));
  });
  document.querySelectorAll(".modal-backdrop").forEach((modal) => {
    modal.addEventListener("mousedown", (event) => {
      if (event.target === modal) closeModal(modal);
    });
    modal.querySelectorAll("[data-close-modal]").forEach((button) => {
      button.addEventListener("click", () => closeModal(modal));
    });
  });
  document.addEventListener("keydown", (event) => {
    const open = document.querySelector(".modal-backdrop.is-open");
    if (event.key === "Escape" && open) {
      closeModal(open);
      return;
    }
    if (event.key !== "Tab" || !open) return;
    const focusable = focusableIn(open);
    if (!focusable.length) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  });

  const requestedModal = params.get("modal");
  if (requestedModal) openModal(requestedModal);

  // Demo file picker. The real Tauri build will use the native dialog.
  const fileInput = document.querySelector("[data-file-input]");
  document.querySelectorAll("[data-add-files]").forEach((button) => {
    button.addEventListener("click", () => fileInput?.click());
  });
  fileInput?.addEventListener("change", () => {
    const count = fileInput.files?.length || 0;
    if (count) showToast(`Добавлено файлов: ${count}. В прототипе очередь не меняется.`);
    fileInput.value = "";
  });

  // Drag & drop is only visual in these mockups.
  document.querySelectorAll("[data-dropzone]").forEach((zone) => {
    zone.addEventListener("click", (event) => {
      if (event.target.closest("[data-add-files]")) return;
      fileInput?.click();
    });
    ["dragenter", "dragover"].forEach((name) => {
      zone.addEventListener(name, (event) => {
        event.preventDefault();
        zone.classList.add("is-dragging");
      });
    });
    ["dragleave", "drop"].forEach((name) => {
      zone.addEventListener(name, (event) => {
        event.preventDefault();
        zone.classList.remove("is-dragging");
      });
    });
    zone.addEventListener("drop", (event) => {
      const count = event.dataTransfer?.files?.length || 0;
      showToast(count ? `Файлов готово к добавлению: ${count}. Это интерактивный макет.` : "Нужны аудио- или видеофайлы.");
    });
  });

  // Start + pause demo state
  document.querySelectorAll("[data-start]").forEach((button) => {
    button.addEventListener("click", () => {
      goTo("transcribe");
      app.classList.add("is-running");
      app.classList.remove("is-paused");
      showToast("Транскрибация запущена. Модель Parakeet V3 работает локально.");
    });
  });

  document.querySelectorAll("[data-pause]").forEach((button) => {
    button.addEventListener("click", () => {
      const paused = app.classList.toggle("is-paused");
      button.querySelectorAll("[data-pause-label]").forEach((label) => {
        label.textContent = paused ? "Продолжить" : "Пауза";
      });
      button.setAttribute("aria-pressed", String(paused));
      const use = button.querySelector("use");
      if (use) {
        const href = use.getAttribute("href") || "";
        use.setAttribute("href", href.replace(/#(pause|play)$/, paused ? "#play" : "#pause"));
      }
      showToast(paused ? "Транскрибация приостановлена." : "Транскрибация продолжена.");
    });
  });

  document.querySelectorAll("[data-export]").forEach((button) => {
    button.addEventListener("click", () => {
      const path = document.querySelector("[data-output-path]")?.textContent?.trim() || "~/Documents/Transaudio";
      showToast(`Готово. 4 файла и расшифровки сохранены в ${path}`);
    });
  });

  document.querySelectorAll("[data-choose-folder]").forEach((button) => {
    button.addEventListener("click", () => {
      showToast("В приложении откроется системный выбор папки.");
    });
  });

  document.querySelectorAll("[data-copy]").forEach((button) => {
    button.addEventListener("click", () => showToast("Текст расшифровки скопирован."));
  });

  document.querySelectorAll("[data-show-toast]").forEach((element) => {
    element.addEventListener("click", () => showToast(element.dataset.showToast));
  });

  // A deterministic, decorative waveform. It carries the real file progress
  // when data-progress is present, so it is more than decoration.
  document.querySelectorAll("[data-waveform]").forEach((waveform) => {
    const count = Number(waveform.dataset.bars || 72);
    const progress = Number(waveform.dataset.progress || 0);
    const played = Math.round((count * progress) / 100);
    const fragment = document.createDocumentFragment();
    for (let index = 0; index < count; index += 1) {
      const bar = document.createElement("i");
      const shape =
        34 +
        Math.abs(Math.sin(index * 0.62)) * 46 +
        Math.abs(Math.cos(index * 0.19)) * 20;
      bar.style.setProperty("--height", `${Math.round(shape)}%`);
      bar.className = index < played ? "is-played" : "";
      fragment.appendChild(bar);
    }
    waveform.appendChild(fragment);
  });

  // First-run permission: a small simulated download inside the modal.
  document.querySelectorAll("[data-permission]").forEach((button) => {
    button.addEventListener("click", () => {
      const modal = button.closest(".modal-backdrop");
      const actions = modal?.querySelector("[data-permission-actions]");
      const state = modal?.querySelector("[data-download-state]");
      if (!actions || !state) {
        closeModal(modal);
        showToast("Разрешение сохранено. Модель будет загружена при запуске.");
        return;
      }
      actions.hidden = true;
      state.hidden = false;
      let value = 0;
      const bar = state.querySelector("[data-download-bar]");
      const label = state.querySelector("[data-download-label]");
      const tick = window.setInterval(() => {
        value = Math.min(100, value + 4);
        if (bar) bar.style.setProperty("--value", `${value}%`);
        if (label) label.textContent = `${value}%`;
        if (value >= 100) {
          window.clearInterval(tick);
          window.setTimeout(() => {
            closeModal(modal);
            showToast("Parakeet V3 загружена. Больше разрешение не потребуется.");
          }, 500);
        }
      }, 90);
    });
  });
})();

(() => {
  "use strict";

  const PROFILE_VALUES = [
    "simple-coding",
    "complex-coding",
    "visual-ui",
    "agentic-tools",
    "large-refactor",
  ];
  const MODEL_ID_PATTERN = /^[^\s/]+\/[^\s/]+$/;
  const MAX_INTERVAL_HOURS = 4294967295;

  const form = document.getElementById("settings-form");
  const autostartInput = document.getElementById("autostart");
  const peakEnterInput = document.getElementById("peak-enter");
  const peakExitInput = document.getElementById("peak-exit");
  const profileSelect = document.getElementById("default-profile");
  const intervalInput = document.getElementById("sync-interval");
  const modelsInput = document.getElementById("ignored-models");
  const saveButton = document.getElementById("save");
  const saveStatus = document.getElementById("save-status");
  const formError = document.getElementById("form-error");
  const loadError = document.getElementById("load-error");
  const loadErrorMessage = document.getElementById("load-error-message");
  const retryButton = document.getElementById("retry-load");

  const controls = [
    autostartInput,
    peakEnterInput,
    peakExitInput,
    profileSelect,
    intervalInput,
    modelsInput,
  ];

  const fieldControls = {
    autostart: [autostartInput],
    notifications: [peakEnterInput, peakExitInput],
    defaultProfile: [profileSelect],
    syncIntervalHours: [intervalInput],
    ignoredModels: [modelsInput],
  };

  const FIELD_ALIASES = {
    peakEnter: "notifications",
    peakExit: "notifications",
    "notifications.peakEnter": "notifications",
    "notifications.peakExit": "notifications",
    "notifications.peak_enter": "notifications",
    "notifications.peak_exit": "notifications",
    default_profile: "defaultProfile",
    sync_interval_hours: "syncIntervalHours",
    ignored_models: "ignoredModels",
  };

  const errorElements = new Map();
  document.querySelectorAll("[data-error-for]").forEach((element) => {
    errorElements.set(element.dataset.errorFor, element);
  });

  let loaded = false;
  let loading = false;
  let saving = false;

  function invokeCommand(command, args) {
    const tauri = window.__TAURI__;
    if (!tauri || !tauri.core || typeof tauri.core.invoke !== "function") {
      return Promise.reject(
        new Error("The Cost Guard desktop bridge is unavailable. Open this page inside the app.")
      );
    }
    return args === undefined
      ? window.__TAURI__.core.invoke(command)
      : window.__TAURI__.core.invoke(command, args);
  }

  function normalizeField(field) {
    if (typeof field !== "string") {
      return null;
    }
    const trimmed = field.trim();
    if (!trimmed) {
      return null;
    }
    if (Object.prototype.hasOwnProperty.call(FIELD_ALIASES, trimmed)) {
      return FIELD_ALIASES[trimmed];
    }
    return Object.prototype.hasOwnProperty.call(fieldControls, trimmed) ? trimmed : null;
  }

  function clearFieldError(field) {
    const element = errorElements.get(field);
    if (element) {
      element.textContent = "";
      element.hidden = true;
    }
    for (const control of fieldControls[field] || []) {
      control.removeAttribute("aria-invalid");
    }
  }

  function clearFieldErrors() {
    for (const field of errorElements.keys()) {
      clearFieldError(field);
    }
  }

  function showFieldError(field, message) {
    const element = errorElements.get(field);
    if (!element) {
      return false;
    }
    element.textContent = message;
    element.hidden = false;
    for (const control of fieldControls[field] || []) {
      control.setAttribute("aria-invalid", "true");
    }
    return true;
  }

  function focusField(field) {
    const control = fieldControls[field] && fieldControls[field][0];
    if (control && !control.disabled && typeof control.focus === "function") {
      control.focus();
    }
  }

  function showFormError(message) {
    formError.textContent = message;
    formError.hidden = false;
  }

  function hideFormError() {
    formError.textContent = "";
    formError.hidden = true;
  }

  function showSaveStatus(message) {
    saveStatus.textContent = message;
    saveStatus.hidden = false;
  }

  function hideSaveStatus() {
    saveStatus.textContent = "";
    saveStatus.hidden = true;
  }

  function showLoadError(message) {
    loadErrorMessage.textContent = message;
    loadError.hidden = false;
  }

  function hideLoadError() {
    loadErrorMessage.textContent = "";
    loadError.hidden = true;
  }

  function updateBusyState() {
    const busy = loading || saving;
    const locked = busy || !loaded;

    for (const control of controls) {
      control.disabled = locked;
    }
    saveButton.disabled = locked;
    saveButton.textContent = saving ? "Saving…" : "Save changes";
    retryButton.disabled = loading;
    form.setAttribute("aria-busy", busy ? "true" : "false");
  }

  function applySettings(settings) {
    const dto = settings && typeof settings === "object" ? settings : {};
    const notifications =
      dto.notifications && typeof dto.notifications === "object" ? dto.notifications : {};
    const interval = Number(dto.syncIntervalHours);

    autostartInput.checked = dto.autostart === true;
    peakEnterInput.checked = notifications.peakEnter === true;
    peakExitInput.checked = notifications.peakExit === true;
    profileSelect.value =
      PROFILE_VALUES.indexOf(dto.defaultProfile) >= 0 ? dto.defaultProfile : "simple-coding";
    intervalInput.value =
      Number.isInteger(interval) && interval >= 1 ? String(interval) : "6";
    modelsInput.value = Array.isArray(dto.ignoredModels)
      ? dto.ignoredModels.filter((model) => typeof model === "string").join("\n")
      : "";
  }

  function parseModels(text) {
    return text
      .split(/\r\n|\r|\n/)
      .map((line) => line.trim())
      .filter((line) => line.length > 0);
  }

  function readForm() {
    return {
      autostart: autostartInput.checked,
      notifications: {
        peakEnter: peakEnterInput.checked,
        peakExit: peakExitInput.checked,
      },
      defaultProfile: profileSelect.value,
      syncIntervalHours: Number(intervalInput.value.trim()),
      ignoredModels: parseModels(modelsInput.value),
    };
  }

  function validate(settings, intervalText) {
    const errors = [];

    if (!/^\d+$/.test(intervalText)) {
      errors.push({
        field: "syncIntervalHours",
        message: "Enter a whole number of hours (1 or more).",
      });
    } else {
      const hours = Number(intervalText);
      if (hours < 1) {
        errors.push({
          field: "syncIntervalHours",
          message: "The sync interval must be at least 1 hour.",
        });
      } else if (hours > MAX_INTERVAL_HOURS) {
        errors.push({
          field: "syncIntervalHours",
          message: "The sync interval must be 4294967295 hours or less.",
        });
      }
    }

    if (PROFILE_VALUES.indexOf(settings.defaultProfile) === -1) {
      errors.push({
        field: "defaultProfile",
        message: "Choose one of the available profiles.",
      });
    }

    const invalidModel = settings.ignoredModels.find((model) => !MODEL_ID_PATTERN.test(model));
    if (invalidModel) {
      errors.push({
        field: "ignoredModels",
        message:
          '"' +
          invalidModel +
          '" is not a provider/model id. Use one id per line, like openai/gpt-5.',
      });
    }

    return errors;
  }

  function rejectionDetails(error) {
    if (error && typeof error === "object") {
      return {
        field: typeof error.field === "string" ? error.field : "",
        message: typeof error.message === "string" ? error.message : "",
      };
    }
    if (typeof error === "string") {
      try {
        const parsed = JSON.parse(error);
        if (parsed && typeof parsed === "object") {
          return {
            field: typeof parsed.field === "string" ? parsed.field : "",
            message: typeof parsed.message === "string" ? parsed.message : "",
          };
        }
      } catch (ignored) {
        // The rejection was a plain string, not serialized JSON.
      }
      return { field: "", message: error };
    }
    return { field: "", message: "" };
  }

  function messageFrom(error, fallback) {
    return rejectionDetails(error).message || fallback;
  }

  async function loadSettings() {
    loading = true;
    loaded = false;
    hideLoadError();
    clearFieldErrors();
    hideFormError();
    hideSaveStatus();
    updateBusyState();

    try {
      const settings = await invokeCommand("settings_get");
      applySettings(settings);
      loaded = true;
    } catch (error) {
      showLoadError(messageFrom(error, "Could not load settings."));
    } finally {
      loading = false;
      updateBusyState();
    }
  }

  function registerEdit(control, field) {
    const handleEdit = () => {
      if (!loaded) {
        return;
      }
      clearFieldError(field);
      hideFormError();
      hideSaveStatus();
    };
    control.addEventListener("input", handleEdit);
    control.addEventListener("change", handleEdit);
  }

  async function handleSubmit(event) {
    event.preventDefault();
    if (!loaded || loading || saving) {
      return;
    }

    clearFieldErrors();
    hideFormError();
    hideSaveStatus();

    const intervalText = intervalInput.value.trim();
    const settings = readForm();
    const errors = validate(settings, intervalText);

    if (errors.length > 0) {
      for (const error of errors) {
        showFieldError(error.field, error.message);
      }
      focusField(errors[0].field);
      return;
    }

    saving = true;
    updateBusyState();

    let fieldToFocus = "";

    try {
      await invokeCommand("settings_set", { settings });
      intervalInput.value = String(settings.syncIntervalHours);
      modelsInput.value = settings.ignoredModels.join("\n");
      showSaveStatus("Settings saved.");
    } catch (error) {
      const details = rejectionDetails(error);
      const field = normalizeField(details.field);
      const message = details.message || "This value is invalid.";

      if (field) {
        showFieldError(field, message);
        fieldToFocus = field;
      } else {
        showFormError(details.message || "Could not save settings.");
      }
    } finally {
      saving = false;
      updateBusyState();
      if (fieldToFocus) {
        focusField(fieldToFocus);
      }
    }
  }

  registerEdit(autostartInput, "autostart");
  registerEdit(peakEnterInput, "notifications");
  registerEdit(peakExitInput, "notifications");
  registerEdit(profileSelect, "defaultProfile");
  registerEdit(intervalInput, "syncIntervalHours");
  registerEdit(modelsInput, "ignoredModels");

  retryButton.addEventListener("click", () => {
    void loadSettings();
  });
  form.addEventListener("submit", handleSubmit);

  void loadSettings();
})();

const initialColors = (() => {
    try {
        const colors = window.__VERGE_INITIAL_THEME_COLORS;
        if (colors && typeof colors === "object") {
            const {darkBg, lightBg} = colors;
            if (typeof darkBg === "string" && typeof lightBg === "string") {
                return {darkBg, lightBg};
            }
        }
    } catch (error) {
        console.warn("[Tauri]", error);
    }
    return {darkBg: "#2E303D", lightBg: "#F5F5F5"};
})();

const prefersDark = (() => {
    try {
        return !!window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)")?.matches;
    } catch (error) {
        console.warn("[Tauri]", error);
        return false;
    }
})();

const initialThemeMode = typeof window.__VERGE_INITIAL_THEME_MODE === "string"
    ? window.__VERGE_INITIAL_THEME_MODE
    : "system";

let initialTheme = prefersDark ? "dark" : "light";
if (initialThemeMode === "dark") {
    initialTheme = "dark";
} else if (initialThemeMode === "light") {
    initialTheme = "light";
}

const applyInitialTheme = (theme) => {
    const isDark = theme === "dark";
    const root = document.documentElement;
    const bgColor = isDark ? initialColors.darkBg : initialColors.lightBg;
    const textColor = isDark ? "#ffffff" : "#333";
    if (root) {
        root.dataset.theme = theme;
        root.style.setProperty("--bg-color", bgColor);
        root.style.setProperty("--text-color", textColor);
        root.style.colorScheme = isDark ? "dark" : "light";
        root.style.color = textColor;
    }
    const paintBody = () => {
        if (!document.body) return;
        document.body.style.color = textColor;
    };
    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", paintBody, {once: true});
    } else {
        paintBody();
    }
    try {
        localStorage.setItem("verge-theme-mode-cache", theme);
    } catch (error) {
        console.warn("[Tauri]", error);
    }
    return isDark;
};

applyInitialTheme(initialTheme);

console.log('[Tauri]');
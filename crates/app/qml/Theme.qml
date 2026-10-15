pragma Singleton
import QtQuick

// Design tokens. Every colour, size, font and duration in the UI comes from here,
// so the look can be changed in one place and every style stays consistent.
//
// Styles:
//   studio   – the default: neutral graphite, flat and compact, a classic
//              studio-console feel (inspired by Logic Pro, drawn from scratch)
//   midnight – deep blue-black with a violet accent and soft glows
//   vivid    – pure black with bright, saturated track colours that run from
//              violet through pink and orange to yellow; full-height regions
//              (a Studio variant: same layout, its own colours. Light mode
//              uses Studio's light colours with the vivid tracks.)
// Each style has a dark and a light variant, and the accent can be overridden.
QtObject {
    id: theme

    // Driven by Main.qml from the user's settings.
    property string style: "studio"
    property bool dark: true
    /// "default" uses the style's own accent; otherwise a key from `accents`.
    property string accentChoice: "default"
    /// Base animation length in ms; 0 means animations are off.
    property int motionMs: 120
    /// Shadows, blur and glow. Off in the Lightweight profile.
    property bool effects: true

    /// Studio and Vivid share the flat console layout.
    readonly property bool studio: style !== "midnight"
    readonly property bool vivid: style === "vivid"
    readonly property int fast: motionMs
    readonly property int slow: Math.round(motionMs * 1.7)

    /// Picks a value for the current style and light/dark mode.
    function pick(studioDark, studioLight, midnightDark, midnightLight) {
        return studio ? (dark ? studioDark : studioLight) : (dark ? midnightDark : midnightLight)
    }
    /// Vivid's dark colour, or `other` in every other style and mode.
    function v(vividDark, other) { return vivid && dark ? vividDark : other }

    // Surfaces, from back to front
    readonly property color bg:         v("#000000", pick("#232325", "#E2E2E6", "#0A0B0F", "#F2F3F7"))
    readonly property color panel:      v("#0B0B0D", pick("#2C2C2F", "#F1F1F3", "#0F1116", "#FFFFFF"))
    readonly property color surface:    v("#131316", pick("#323236", "#FAFAFB", "#15171E", "#FFFFFF"))
    readonly property color raised:     v("#1B1B1F", pick("#3A3A3E", "#EAEAED", "#1C1F28", "#F5F6F9"))
    readonly property color sunken:     v("#060607", pick("#19191B", "#D6D6DB", "#0C0E12", "#ECEEF3"))
    readonly property color hover:      v("#26262B", pick("#45454A", "#DDDDE2", "#242833", "#E9EBF0"))
    readonly property color line:       v("#1A1A1E", pick("#161618", "#C3C3C9", "#242833", "#E0E3EA"))
    readonly property color lineStrong: v("#303036", pick("#4B4B50", "#ACACB3", "#353A48", "#C9CED8"))

    // Text
    readonly property color text:      v("#F4F4F5", pick("#E8E8EA", "#1C1C1E", "#EDEFF5", "#12141A"))
    readonly property color textDim:   v("#A1A1AA", pick("#A3A3A8", "#56565C", "#9BA2B4", "#585F70"))
    readonly property color textFaint: v("#63636B", pick("#6F6F75", "#8E8E94", "#5E6576", "#9399A8"))

    // Accent: the style's own colour unless the user picked one.
    readonly property var accents: ({
        blue:   { dark: "#3D8EF0", light: "#2272DB", label: "Blue" },
        violet: { dark: "#7C6CFF", light: "#5A48EA", label: "Violet" },
        teal:   { dark: "#2EC4B6", light: "#0E9488", label: "Teal" },
        green:  { dark: "#4CC25A", light: "#2A9A3A", label: "Green" },
        orange: { dark: "#F2994A", light: "#D2701A", label: "Orange" },
        pink:   { dark: "#F06292", light: "#D63B72", label: "Pink" },
        red:    { dark: "#F2564B", light: "#D23228", label: "Red" }
    })
    readonly property string accentKey: accentChoice !== "default" && accents[accentChoice] !== undefined
                                        ? accentChoice : (studio && !vivid ? "blue" : "violet")
    readonly property color accent: dark ? accents[accentKey].dark : accents[accentKey].light
    readonly property color accentHover: Qt.lighter(accent, dark ? 1.12 : 0.92)
    readonly property color accentText:  "#FFFFFF"
    readonly property color accentSoft:  tint(accent, dark ? 0.18 : 0.12)
    readonly property color accent2:     pick("#5AC8FA", "#1A93C9", "#34D5C4", "#0EA394")

    // Status
    readonly property color success: dark ? "#4CD07A" : "#169C45"
    readonly property color warning: dark ? "#F2C14E" : "#B57A12"
    readonly property color danger:  dark ? "#F2564B" : "#D23228"

    // Transport. Studio uses the classic console colours: green play, yellow cycle.
    readonly property color playIdle:      studio ? raised : accent
    readonly property color playIdleIcon:  studio ? text : accentText
    readonly property color playActive:    studio ? (dark ? "#4CC25A" : "#2E9E3C") : accent
    readonly property color loopOn:        studio ? (dark ? "#E9C440" : "#B8920F") : accent
    readonly property color record:        dark ? "#F2453D" : "#D42A22"
    /// Markers (Intro, Verse…) on the ruler: warm orange in every style.
    readonly property color marker:        dark ? "#F0A04B" : "#E08A2C"

    // The time display, styled like a hardware LCD in every style.
    readonly property color lcd:       vivid ? sunken : studio ? "#141416" : sunken
    readonly property color lcdText:   studio ? "#ECECEE" : text
    readonly property color lcdDim:    studio ? "#7C7C82" : textFaint

    // Regions (clips) on the timeline
    readonly property color region:        studio ? (dark ? "#3F7DC1" : "#5A93D2") : tint(accent, dark ? 0.10 : 0.07)
    readonly property color regionHeader:  studio ? Qt.darker(region, 1.25) : tint(accent, dark ? 0.22 : 0.14)
    readonly property color regionBorder:  studio ? Qt.darker(region, 1.5) : tint(accent, 0.45)
    readonly property color regionText:    studio ? "#FFFFFF" : (dark ? text : accent)
    readonly property color waveIdle:      studio ? Qt.rgba(0.04, 0.10, 0.18, 0.55) : tint(accent, dark ? 0.62 : 0.5)
    readonly property color wavePlayed:    studio ? Qt.rgba(0.02, 0.06, 0.12, 0.92) : accent
    readonly property color playhead:      studio ? "#FFFFFF" : text

    // Track colours, keyed by name (stored in the project file).
    // Vivid keeps the same names but turns the default order (blue, green,
    // yellow…) into a warm gradient, so new tracks run violet → pink → orange → yellow.
    readonly property var vividTrackColors: ({
        blue:   dark ? "#A55CF5" : "#9146E8",
        green:  dark ? "#D35FD6" : "#C04BC4",
        yellow: dark ? "#EC5D92" : "#DC4680",
        orange: dark ? "#F27141" : "#E05A28",
        red:    dark ? "#F59A3C" : "#E2841F",
        pink:   dark ? "#F5C543" : "#D9A821",
        violet: dark ? "#5C8DFF" : "#3F74F0",
        teal:   dark ? "#3FD3C6" : "#1FB3A6"
    })
    readonly property var classicTrackColors: ({
        blue:   dark ? "#3F7DC1" : "#5A93D2",
        green:  dark ? "#3E9B5A" : "#55B06F",
        yellow: dark ? "#B8962F" : "#D4B04A",
        orange: dark ? "#C2702F" : "#DE8A47",
        red:    dark ? "#B9474A" : "#D4605F",
        pink:   dark ? "#B54C86" : "#D166A0",
        violet: dark ? "#7158C2" : "#8B74D8",
        teal:   dark ? "#2E9A92" : "#45B2A9"
    })
    readonly property var trackColors: vivid ? vividTrackColors : classicTrackColors
    readonly property var trackColorNames: ["blue", "green", "yellow", "orange", "red", "pink", "violet", "teal"]
    function trackColor(name) {
        const c = trackColors
        if (!c) return "#3F7DC1"   // only while the theme is still being set up
        return c[name] !== undefined ? c[name] : c.blue
    }

    readonly property color overlay: dark ? Qt.rgba(0, 0, 0, 0.6) : Qt.rgba(0.06, 0.07, 0.12, 0.3)
    readonly property color shadow:  dark ? Qt.rgba(0, 0, 0, 0.6) : Qt.rgba(0.1, 0.12, 0.2, 0.16)
    /// Floating panels (mixer, ⌘K, menus) are frosted glass: they blur what's
    /// behind them. Off with effects (Lightweight profile), where they're solid.
    readonly property bool glass: effects
    /// The two colours that glow inside glass panels.
    readonly property color glowA: vivid ? "#A55CF5" : accent
    readonly property color glowB: vivid ? "#F27141" : studio ? (dark ? "#2EC4B6" : "#0E9488") : accent2
    /// Soft colour glows behind glass, in every style, only with effects on.
    readonly property bool glows: effects
    /// The window's content, which popups (menus, dropdowns) blur behind them.
    /// Set by Main.qml.
    property Item backdrop: null

    // Shape: Studio is flatter and tighter.
    readonly property int radiusSm: vivid ? 4 : studio ? 3 : 6
    readonly property int radiusMd: vivid ? 6 : studio ? 5 : 9
    readonly property int radiusLg: vivid ? 10 : studio ? 7 : 14
    readonly property int radiusXl: vivid ? 16 : studio ? 10 : 20

    // Type scale (pixels)
    readonly property int textXs: 11
    readonly property int textSm: 12
    readonly property int textMd: 13
    readonly property int textLg: 15
    readonly property int textXl: 19
    readonly property int textXxl: 24

    // Bundled open-source fonts: Inter and JetBrains Mono (SIL OFL), Phosphor icons (MIT)
    readonly property string fontDir: "qrc:/qt/qml/Isai/assets/fonts/"
    property FontLoader interRegular: FontLoader { source: theme.fontDir + "Inter-Regular.ttf" }
    property FontLoader interSemiBold: FontLoader { source: theme.fontDir + "Inter-SemiBold.ttf" }
    property FontLoader monoLoader: FontLoader { source: theme.fontDir + "JetBrainsMono-Medium.ttf" }
    property FontLoader iconLoader: FontLoader { source: theme.fontDir + "Phosphor.ttf" }
    property FontLoader iconFillLoader: FontLoader { source: theme.fontDir + "Phosphor-Fill.ttf" }

    readonly property string font: interRegular.name
    readonly property string mono: monoLoader.name
    readonly property string iconFont: iconLoader.name
    readonly property string iconFillFont: iconFillLoader.name

    function tint(c, alpha) {
        return Qt.rgba(c.r, c.g, c.b, alpha)
    }
}

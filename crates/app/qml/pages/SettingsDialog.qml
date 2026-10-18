import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// Settings: a sidebar of sections and a scrolling list of options.
// Built as an overlay inside the window (not a native popup) so its look and
// motion are fully ours.
Item {
    id: dialog
    /// What the frosted glass blurs: the page behind the dialog.
    property Item backdrop: null
    property var app
    property int section: 0
    property bool shown: false

    function open() { shown = true; panel.forceActiveFocus() }
    function close() { shown = false }

    readonly property var sections: [
        { glyph: Icons.gauge, title: "Performance", subtitle: "Choose a profile, or fine-tune each option." },
        { glyph: Icons.palette, title: "Appearance", subtitle: "Style, colours, motion and visual detail." },
        { glyph: Icons.speakerHigh, title: "Audio", subtitle: "Output, input, buffer and metronome." },
        { glyph: Icons.magicWand, title: "Helpers", subtitle: "Tips the app can give you." },
        { glyph: Icons.info, title: "General", subtitle: "View, setup and about." }
    ]

    anchors.fill: parent
    visible: opacity > 0
    opacity: shown ? 1 : 0
    Behavior on opacity { NumberAnimation { duration: dialog.shown ? Theme.slow : Theme.fast } }

    Shortcut { sequence: "Escape"; enabled: dialog.shown; onActivated: dialog.close() }

    // Dimmed backdrop; clicking it closes the dialog.
    Rectangle {
        anchors.fill: parent
        color: Theme.overlay
        MouseArea { anchors.fill: parent; onClicked: dialog.close() }
    }

    GlassPanel {
        backdrop: dialog.backdrop
        clarity: 0.3
        id: panel
        radius: Theme.radiusXl
        anchors.centerIn: parent
        width: Math.min(860, parent.width - 48)
        height: Math.min(600, parent.height - 48)
        scale: dialog.shown ? 1 : 0.96
        Behavior on scale { NumberAnimation { duration: Theme.slow; easing.type: Easing.OutCubic } }
        focus: true

        // Swallow clicks so they don't reach the backdrop.
        MouseArea { anchors.fill: parent }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 1
        spacing: 0

        // Sidebar
        Rectangle {
            Layout.preferredWidth: 220
            Layout.fillHeight: true
            color: Theme.tint(Theme.panel, Theme.glass ? 0.35 : 1)
            topLeftRadius: Theme.radiusXl
            bottomLeftRadius: Theme.radiusXl
            border.width: 0

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 14
                anchors.topMargin: 20
                spacing: 2

                DText {
                    Layout.leftMargin: 8
                    Layout.bottomMargin: 14
                    text: "Settings"
                    font.pixelSize: Theme.textXl
                    font.weight: Font.DemiBold
                }

                Repeater {
                    model: dialog.sections
                    delegate: AbstractButton {
                        id: nav
                        required property var modelData
                        required property int index
                        readonly property bool current: dialog.section === index
                        Layout.fillWidth: true
                        implicitHeight: 38
                        hoverEnabled: true
                        focusPolicy: Qt.StrongFocus
                        onClicked: dialog.section = index

                        background: Rectangle {
                            radius: Theme.radiusMd
                            color: nav.current ? Theme.accentSoft : nav.hovered ? Theme.raised : "transparent"
                            Behavior on color { ColorAnimation { duration: Theme.fast } }
                            FocusRing { visible: nav.visualFocus }
                        }
                        contentItem: RowLayout {
                            spacing: 10
                            Icon {
                                Layout.leftMargin: 10
                                glyph: nav.modelData.glyph
                                size: 17
                                filled: nav.current
                                color: nav.current ? Theme.accent : Theme.textDim
                            }
                            DText {
                                Layout.fillWidth: true
                                text: nav.modelData.title
                                strong: nav.current
                                color: nav.current ? Theme.text : Theme.textDim
                            }
                        }
                    }
                }

                Item { Layout.fillHeight: true }
                DText {
                    Layout.leftMargin: 8
                    text: "Version 0.0.1 · Phase 0"
                    faint: true
                    font.pixelSize: Theme.textXs
                }
            }
        }

        Rectangle { Layout.preferredWidth: 1; Layout.fillHeight: true; color: Theme.line }

        // Content
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 92
                ColumnLayout {
                    anchors.left: parent.left
                    anchors.right: closeButton.left
                    anchors.leftMargin: 28
                    anchors.rightMargin: 12
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 3
                    DText {
                        text: dialog.sections[dialog.section].title
                        font.pixelSize: Theme.textXl
                        font.weight: Font.DemiBold
                    }
                    DText { text: dialog.sections[dialog.section].subtitle; dim: true }
                }
                IconButton {
                    id: closeButton
                    anchors.right: parent.right
                    anchors.rightMargin: 16
                    anchors.top: parent.top
                    anchors.topMargin: 16
                    glyph: Icons.x
                    tip: "Close (Esc)"
                    onClicked: dialog.close()
                }
            }

            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                contentWidth: availableWidth
                clip: true

                StackLayout {
                    x: 28
                    width: parent.width - 56
                    currentIndex: dialog.section

                    // Performance
                    ColumnLayout {
                        spacing: 10
                        Banner {
                            Layout.fillWidth: true
                            visible: dialog.app.profile === "custom"
                            tone: "info"
                            glyph: Icons.wrench
                            text: "You're using a custom mix of options. Pick a profile to reset them."
                        }
                        Repeater {
                            model: [
                                { key: "smooth", glyph: Icons.rocket, title: "Smooth", text: "Every animation and effect. For modern computers." },
                                { key: "balanced", glyph: Icons.scales, title: "Balanced", text: "Short animations, medium detail. Most laptops." },
                                { key: "lightweight", glyph: Icons.feather, title: "Lightweight", text: "No animations, simple visuals. Older or low-spec computers." }
                            ]
                            delegate: ChoiceCard {
                                required property var modelData
                                Layout.fillWidth: true
                                glyph: modelData.glyph
                                title: modelData.title
                                description: modelData.text
                                selected: dialog.app.profile === modelData.key
                                badge: dialog.app.recommended === modelData.key ? "Recommended" : ""
                                onClicked: dialog.app.setText("profile", modelData.key)
                            }
                        }
                    }

                    // Appearance
                    ColumnLayout {
                        spacing: 0
                        SettingRow {
                            title: "Style"
                            description: "Studio is a classic console look. Midnight is deep violet with soft glows. Vivid is pure black with bright track colours."
                            Segmented {
                                options: [
                                    { value: "studio", label: "Studio", glyph: Icons.sliders },
                                    { value: "midnight", label: "Midnight", glyph: Icons.sparkle },
                                    { value: "vivid", label: "Vivid", glyph: Icons.palette }
                                ]
                                value: dialog.app.uiStyle
                                onActivated: v => dialog.app.setText("uiStyle", v)
                            }
                        }
                        SettingRow {
                            title: "Accent colour"
                            description: "Used for buttons, selection and highlights."
                            AccentPicker {
                                value: dialog.app.accent
                                onPicked: v => dialog.app.setText("accent", v)
                            }
                        }
                        SettingRow {
                            title: "Interface size"
                            description: "Makes buttons, text and panels bigger or smaller. Smaller fits more on a small screen."
                            Segmented {
                                options: [
                                    { value: 75, label: "75%" },
                                    { value: 100, label: "100%" },
                                    { value: 125, label: "125%" },
                                    { value: 150, label: "150%" },
                                    { value: 200, label: "200%" }
                                ]
                                value: dialog.app.uiScale
                                onActivated: v => dialog.app.setInt("uiScale", v)
                            }
                        }
                        SettingRow {
                            title: "Theme"
                            description: "Follow your system, or always light or dark."
                            Segmented {
                                options: [
                                    { value: "system", label: "System", glyph: Icons.circleHalf },
                                    { value: "dark", label: "Dark", glyph: Icons.moon },
                                    { value: "light", label: "Light", glyph: Icons.sun }
                                ]
                                value: dialog.app.theme
                                onActivated: v => dialog.app.setText("theme", v)
                            }
                        }
                        SettingRow {
                            title: "Animations"
                            description: "Motion when things open, move and change."
                            Segmented {
                                options: [
                                    { value: "full", label: "Full" },
                                    { value: "reduced", label: "Reduced" },
                                    { value: "off", label: "Off" }
                                ]
                                value: dialog.app.animations
                                onActivated: v => dialog.app.setText("animations", v)
                            }
                        }
                        SettingRow {
                            title: "Frame rate while playing"
                            description: "How often the screen updates during playback. Lower saves power."
                            Segmented {
                                options: [
                                    { value: 30, label: "30" },
                                    { value: 60, label: "60" },
                                    { value: 120, label: "120" },
                                    { value: 0, label: "Display" }
                                ]
                                value: dialog.app.uiFps
                                onActivated: v => dialog.app.setInt("uiFps", v)
                            }
                        }
                        SettingRow {
                            title: "Level meter updates"
                            description: "Updates per second for the volume meters."
                            Segmented {
                                options: [
                                    { value: 15, label: "15" },
                                    { value: 30, label: "30" },
                                    { value: 60, label: "60" }
                                ]
                                value: dialog.app.meterHz
                                onActivated: v => dialog.app.setInt("meterHz", v)
                            }
                        }
                        SettingRow {
                            title: "Waveform detail"
                            description: "Fewer, wider bars draw faster on weak graphics."
                            Segmented {
                                options: [
                                    { value: "low", label: "Low" },
                                    { value: "medium", label: "Medium" },
                                    { value: "high", label: "High" }
                                ]
                                value: dialog.app.waveformDetail
                                onActivated: v => dialog.app.setText("waveformDetail", v)
                            }
                        }
                        SettingRow {
                            title: "Shadows and glow"
                            description: "Soft shadows, blur and glow effects."
                            divider: false
                            Toggle {
                                checked: dialog.app.effects
                                onToggled: dialog.app.setBool("effects", checked)
                            }
                        }
                    }

                    // Audio
                    ColumnLayout {
                        spacing: 0
                        SettingRow {
                            title: "Output device"
                            description: "Speakers, headphones or an audio interface."
                            RowLayout {
                                spacing: 6
                                DevicePicker { app: dialog.app; implicitWidth: 240 }
                                IconButton { glyph: Icons.arrowsClockwise; tip: "Look for new devices"; onClicked: dialog.app.refreshDevices() }
                            }
                        }
                        SettingRow {
                            title: "Buffer size"
                            description: "Bigger stops crackling on slower computers. " + settingsBuffer.latencyText() + "."
                            BufferPicker { id: settingsBuffer; app: dialog.app }
                        }
                        SettingRow {
                            title: "Input"
                            description: "The microphone or audio interface you record from."
                            RowLayout {
                                spacing: 6
                                Dropdown {
                                    implicitWidth: 240
                                    model: {
                                        const names = ["System default"]
                                        for (let i = 0; i < dialog.app.inputDeviceNames.length; ++i)
                                            names.push(dialog.app.inputDeviceNames[i])
                                        return names
                                    }
                                    currentIndex: {
                                        if (dialog.app.inputDeviceId === "") return 0
                                        for (let i = 0; i < dialog.app.inputDeviceIds.length; ++i)
                                            if (dialog.app.inputDeviceIds[i] === dialog.app.inputDeviceId) return i + 1
                                        return 0
                                    }
                                    onActivated: index => dialog.app.setText("inputDevice", index === 0 ? "" : dialog.app.inputDeviceIds[index - 1])
                                }
                                IconButton { glyph: Icons.arrowsClockwise; tip: "Look for new devices"; onClicked: dialog.app.refreshInputs() }
                            }
                        }
                        SettingRow {
                            title: "Input channels"
                            description: "One channel for a microphone or guitar; a stereo pair for keyboards or mixers."
                            Segmented {
                                options: [
                                    { value: "m0", label: "1" },
                                    { value: "m1", label: "2" },
                                    { value: "s0", label: "1 + 2" }
                                ]
                                value: (dialog.app.inputStereo ? "s" : "m") + dialog.app.inputChannel
                                onActivated: v => {
                                    dialog.app.setBool("inputStereo", v[0] === "s")
                                    dialog.app.setInt("inputChannel", parseInt(v.slice(1)))
                                }
                            }
                        }
                        SettingRow {
                            title: "Recording latency correction"
                            description: "If takes sound late against the beat, increase this. Positive values move recordings earlier."
                            RowLayout {
                                spacing: 10
                                DSlider {
                                    implicitWidth: 180
                                    from: -100; to: 100; stepSize: 1
                                    value: dialog.app.latencyOffsetMs
                                    onMoved: dialog.app.setInt("latencyOffset", Math.round(value))
                                }
                                DText {
                                    Layout.preferredWidth: 52
                                    text: (dialog.app.latencyOffsetMs > 0 ? "+" : "") + dialog.app.latencyOffsetMs + " ms"
                                    font.family: Theme.mono
                                    dim: true
                                }
                            }
                        }
                        SettingRow {
                            title: "Metronome volume"
                            description: "How loud the click is, compared with normal."
                            RowLayout {
                                spacing: 10
                                DSlider {
                                    implicitWidth: 180
                                    from: 0; to: 150; stepSize: 5
                                    value: dialog.app.clickVolume
                                    onMoved: dialog.app.setInt("clickVolume", Math.round(value))
                                }
                                DText {
                                    Layout.preferredWidth: 40
                                    text: dialog.app.clickVolume + "%"
                                    font.family: Theme.mono
                                    dim: true
                                }
                            }
                        }
                        SettingRow {
                            title: "Count-in length"
                            description: "Bars the metronome plays before playback starts, when count-in is on."
                            Segmented {
                                options: [
                                    { value: 1, label: "1 bar" },
                                    { value: 2, label: "2 bars" }
                                ]
                                value: dialog.app.countInBars
                                onActivated: v => dialog.app.setInt("countInBars", v)
                            }
                        }
                        SettingRow {
                            title: "Test sound"
                            description: dialog.app.audioOk
                                         ? dialog.app.deviceName + " · " + dialog.app.sampleRate + " Hz"
                                         : (dialog.app.audioError || "Sound is off")
                            divider: false
                            DButton {
                                enabled: dialog.app.audioOk
                                glyph: Icons.speakerHigh
                                text: "Play test sound"
                                onClicked: dialog.app.testTone()
                            }
                        }
                    }

                    // Helpers
                    ColumnLayout {
                        spacing: 0
                        SettingRow {
                            title: "Eco mode"
                            description: "Instruments play in their lightest quality (piano Light, less synth unison), so older computers keep up. Exports are always full quality."
                            Toggle {
                                checked: dialog.app.eco
                                onToggled: dialog.app.setBool("eco", checked)
                            }
                        }
                        SettingRow {
                            title: "Suggest a lighter profile"
                            description: "If the audio starts crackling, offer to switch to Lightweight."
                            divider: false
                            Toggle {
                                checked: dialog.app.suggestLighter
                                onToggled: dialog.app.setBool("suggestLighter", checked)
                            }
                        }
                    }

                    // General
                    ColumnLayout {
                        spacing: 0
                        SettingRow {
                            title: "View"
                            description: "Simple hides technical details. Pro shows everything."
                            Segmented {
                                options: [
                                    { value: "simple", label: "Simple", glyph: Icons.sparkle },
                                    { value: "pro", label: "Pro", glyph: Icons.sliders }
                                ]
                                value: dialog.app.view
                                onActivated: v => dialog.app.setText("view", v)
                            }
                        }
                        SettingRow {
                            title: "First-launch setup"
                            description: "Measure your computer again and walk through setup."
                            DButton {
                                glyph: Icons.arrowCounterClockwise
                                text: "Run setup again"
                                onClicked: { dialog.close(); dialog.app.rerunSetup() }
                            }
                        }
                        SettingRow {
                            title: "Settings file"
                            description: dialog.app.settingsPath
                            divider: false
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            Layout.topMargin: 12
                            implicitHeight: about.implicitHeight + 32
                            radius: Theme.radiusLg
                            color: Theme.raised
                            border.color: Theme.line
                            ColumnLayout {
                                id: about
                                anchors.fill: parent
                                anchors.margins: 16
                                spacing: 6
                                RowLayout {
                                    spacing: 10
                                    Logo { variant: "lockup"; size: 30 }
                                    DText { text: "Free and open source, forever"; strong: true }
                                }
                                DText {
                                    Layout.fillWidth: true
                                    wrapMode: Text.WordWrap
                                    dim: true
                                    font.pixelSize: Theme.textSm
                                    text: "Licensed under GPL-3.0. Built with Qt (LGPL) and Rust. Icons by Phosphor (MIT). Fonts: Inter and JetBrains Mono (SIL Open Font License)."
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
}

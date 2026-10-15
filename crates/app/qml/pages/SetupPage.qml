import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// First-launch setup: check the computer, pick a profile, set up audio, choose the look.
Item {
    id: page
    property var app
    property int step: 0
    // Nothing is chosen for the user: these stay empty until they pick.
    property string chosenProfile: ""
    property string chosenView: ""

    readonly property bool canContinue: step === 0 ? !app.checking
                                      : step === 1 ? chosenProfile !== ""
                                      : step === 3 ? chosenView !== ""
                                      : true
    readonly property string waitingHint: step === 0 ? "Measuring…"
                                        : step === 1 ? "Pick a profile to continue"
                                        : step === 3 ? "Pick a view to continue"
                                        : ""

    readonly property var titles: [
        "Let's see what your computer can do",
        "Pick how the app should feel",
        "Set up your sound",
        "Make it yours"
    ]

    function go(next) {
        const forward = next > step
        step = next
        stack.replace(null, steps[next], { app: page.app },
                      forward ? StackView.PushTransition : StackView.PopTransition)
    }
    readonly property var steps: [checkStep, profileStep, audioStep, lookStep]

    function next() {
        if (step === 3) {
            app.finishSetup()
        } else {
            app.saveSetupProgress()
            go(step + 1)
        }
    }

    Connections {
        target: page.app
        function onNeedsSetupChanged() {
            if (page.app.needsSetup) {
                page.step = 0
                page.chosenProfile = ""
                page.chosenView = ""
                stack.replace(null, checkStep, { app: page.app })
            }
        }
    }

    // The top of the screen moves the window (there is no title bar on macOS).
    WindowDragArea {
        width: parent.width
        height: 56
    }

    // Colour behind the card, which the glass card blurs.
    Ambient { id: ambient; anchors.fill: parent; z: -1 }

    GlassPanel {
        id: card
        backdrop: ambient
        clarity: 0.35
        radius: Theme.radiusXl
        width: Math.min(640, parent.width - 48)
        height: column.implicitHeight + 64
        anchors.centerIn: parent
        Behavior on height { NumberAnimation { duration: Theme.slow; easing.type: Easing.OutCubic } }

        ColumnLayout {
            id: column
            anchors.fill: parent
            anchors.margins: 32
            spacing: 24

            ColumnLayout {
                spacing: 10
                DText { text: "Welcome to"; dim: true; font.pixelSize: Theme.textLg }
                Logo { variant: "lockup"; size: 58 }
                DText { text: page.titles[page.step]; dim: true }
            }

            StepIndicator {
                Layout.fillWidth: true
                labels: ["Computer", "Profile", "Sound", "Look"]
                current: page.step
            }

            Banner {
                Layout.fillWidth: true
                visible: page.app.setupNotice !== ""
                tone: "warning"
                text: page.app.setupNotice
            }

            StackView {
                id: stack
                Layout.fillWidth: true
                Layout.preferredHeight: currentItem ? currentItem.implicitHeight : 0
                clip: true
                initialItem: checkStep

                Component.onCompleted: if (currentItem) currentItem.app = page.app

                pushEnter: Transition {
                    NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.slow }
                    NumberAnimation { property: "x"; from: 40; to: 0; duration: Theme.slow; easing.type: Easing.OutCubic }
                }
                pushExit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: Theme.fast } }
                popEnter: Transition {
                    NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.slow }
                    NumberAnimation { property: "x"; from: -40; to: 0; duration: Theme.slow; easing.type: Easing.OutCubic }
                }
                popExit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: Theme.fast } }
                replaceEnter: pushEnter
                replaceExit: pushExit
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 10
                DButton {
                    visible: page.step > 1
                    variant: "ghost"
                    glyph: Icons.arrowLeft
                    text: "Back"
                    onClicked: page.go(page.step - 1)
                }
                Item { Layout.fillWidth: true }
                DText {
                    text: page.canContinue ? "You can change all of this later in Settings" : page.waitingHint
                    faint: true
                    font.pixelSize: Theme.textSm
                }
                DButton {
                    variant: "primary"
                    large: true
                    enabled: page.canContinue
                    text: page.step === 3 ? "Start making music" : "Continue"
                    glyph: page.step === 3 ? Icons.sparkle : Icons.arrowRight
                    onClicked: page.next()
                }
            }
        }
    }

    // ------------------------------------------------------------- steps

    Component {
        id: checkStep
        ColumnLayout {
            property var app   // passed by the StackView like every step
            spacing: 16
            RowLayout {
                spacing: 10
                Spinner { size: 20; visible: page.app.checking }
                Icon { glyph: Icons.checkCircle; filled: true; size: 22; color: Theme.success; visible: !page.app.checking }
                DText {
                    text: page.app.checking ? "Measuring real audio performance…" : "All done. Here's your computer"
                    strong: true
                    font.pixelSize: Theme.textLg
                }
            }
            DText {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                dim: true
                text: page.app.checking
                      ? "This takes about a second. We test how much sound your computer can actually process, instead of guessing from its specs."
                      : "Your computer can play about " + page.app.voices.toLocaleString(Qt.locale(), "f", 0)
                        + " synth voices at once. Next, you'll choose how the app should feel."
            }
            Flow {
                Layout.fillWidth: true
                spacing: 8
                InfoChip { glyph: Icons.cpu; text: page.app ? (page.app.cpuName || "Processor") : "" }
                InfoChip { glyph: Icons.gauge; text: page.app.cores + " cores" }
                InfoChip { glyph: Icons.memory; text: Math.round(page.app.ramGb) + " GB memory" }
                InfoChip { visible: !page.app.checking; glyph: Icons.waveform; text: page.app.voices.toLocaleString(Qt.locale(), "f", 0) + " voices" }
            }
        }
    }

    Component {
        id: profileStep
        ColumnLayout {
            property var app
            spacing: 10
            Flow {
                Layout.fillWidth: true
                Layout.bottomMargin: 6
                spacing: 8
                InfoChip { glyph: Icons.cpu; text: app ? (app.cpuName || "Processor") : "" }
                InfoChip { glyph: Icons.memory; text: app ? Math.round(app.ramGb) + " GB" : "" }
                InfoChip { glyph: Icons.waveform; text: app ? app.voices.toLocaleString(Qt.locale(), "f", 0) + " voices in real time" : "" }
            }
            Repeater {
                model: [
                    { key: "smooth", glyph: Icons.rocket, title: "Smooth",
                      text: "Every animation and effect, high frame rate and detailed waveforms. Best on modern computers." },
                    { key: "balanced", glyph: Icons.scales, title: "Balanced",
                      text: "Short animations and medium detail. A good fit for most laptops." },
                    { key: "lightweight", glyph: Icons.feather, title: "Lightweight",
                      text: "No animations and simple visuals, so all the power goes to your music. Ideal for older computers." }
                ]
                delegate: ChoiceCard {
                    required property var modelData
                    Layout.fillWidth: true
                    glyph: modelData.glyph
                    title: modelData.title
                    description: modelData.text
                    selected: page.chosenProfile === modelData.key
                    badge: app && app.recommended === modelData.key ? "Recommended" : ""
                    onClicked: {
                        page.chosenProfile = modelData.key
                        app.setText("profile", modelData.key)
                    }
                }
            }
        }
    }

    Component {
        id: audioStep
        ColumnLayout {
            id: audio
            property var app
            spacing: 0
            onAppChanged: if (app) app.prepareAudio()

            SettingRow {
                title: "Output"
                description: "Where you'll hear your music: speakers, headphones or an audio interface."
                DevicePicker { app: audio.app; implicitWidth: 240 }
            }
            SettingRow {
                id: bufferRow
                title: "Buffer size"
                description: "Bigger stops crackling on slower computers, with a little more delay. " + bufferPicker.latencyText() + "."
                BufferPicker { id: bufferPicker; app: audio.app }
            }
            SettingRow {
                title: "Test your sound"
                description: audio.app && audio.app.audioOk ? "You should hear a short, gentle beep." : "Sound isn't working yet."
                divider: false
                DButton {
                    enabled: audio.app && audio.app.audioOk
                    glyph: Icons.speakerHigh
                    text: "Play test sound"
                    onClicked: audio.app.testTone()
                }
            }
            Banner {
                Layout.fillWidth: true
                Layout.topMargin: 8
                visible: audio.app && audio.app.audioError !== ""
                tone: "danger"
                text: audio.app ? audio.app.audioError + "  You can continue and fix this later." : ""
                DButton { text: "Try again"; glyph: Icons.arrowsClockwise; onClicked: audio.app.retryAudio() }
            }
        }
    }

    Component {
        id: lookStep
        ColumnLayout {
            id: look
            property var app
            spacing: 12
            DText { text: "Choose your view"; strong: true; font.pixelSize: Theme.textLg }
            RowLayout {
                Layout.fillWidth: true
                spacing: 10
                ChoiceCard {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    glyph: Icons.sparkle
                    title: "Simple"
                    description: "Just the essentials. Perfect if you're new to making music."
                    selected: page.chosenView === "simple"
                    onClicked: {
                        page.chosenView = "simple"
                        look.app.setText("view", "simple")
                    }
                }
                ChoiceCard {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    glyph: Icons.sliders
                    title: "Pro"
                    description: "Everything visible, including technical details."
                    selected: page.chosenView === "pro"
                    onClicked: {
                        page.chosenView = "pro"
                        look.app.setText("view", "pro")
                    }
                }
            }
            SettingRow {
                title: "Style"
                description: "Studio is a classic console look. Midnight is deep violet with soft glows. Vivid is pure black with bright track colours."
                Segmented {
                    options: [
                        { value: "studio", label: "Studio", glyph: Icons.sliders },
                        { value: "midnight", label: "Midnight", glyph: Icons.sparkle },
                        { value: "vivid", label: "Vivid", glyph: Icons.palette }
                    ]
                    value: look.app ? look.app.uiStyle : "studio"
                    onActivated: v => look.app.setText("uiStyle", v)
                }
            }
            SettingRow {
                title: "Theme"
                description: "Follow your system, or always light or dark."
                divider: false
                Segmented {
                    options: [
                        { value: "system", label: "System", glyph: Icons.circleHalf },
                        { value: "dark", label: "Dark", glyph: Icons.moon },
                        { value: "light", label: "Light", glyph: Icons.sun }
                    ]
                    value: look.app ? look.app.theme : "system"
                    onActivated: v => look.app.setText("theme", v)
                }
            }
        }
    }
}

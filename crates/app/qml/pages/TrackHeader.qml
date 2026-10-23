import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// One track's header: colour, name, mute/solo, volume and pan.
// Double-click the name to rename; right-click for more; drag the header
// up or down to reorder tracks.
Rectangle {
    id: header
    property var app
    property var track          // one entry of app.tracksJson
    property bool selected: false
    property int index: 0
    property int count: 1
    property real rowHeight: 84
    /// Post-fader [left, right] level, 0..1 on the meter scale.
    property var level: [0, 0]
    /// The instrument icon was clicked: open the instrument's window.
    signal instrumentRequested()

    // Reordering: the header follows the mouse; the drop slot is shown by `target`.
    property real dragY: 0
    property bool dragging: false
    readonly property int target: Math.max(0, Math.min(count - 1, index + Math.round(dragY / rowHeight)))
    transform: Translate { y: header.dragY }
    z: dragging ? 10 : 0

    readonly property color tint: Theme.trackColor(track.color)

    // A flat row, like a DAW's track list: a hairline separates it from the next track.
    radius: 0
    color: dragging ? Theme.raised : selected ? Theme.tint(tint, Theme.dark ? 0.12 : 0.09) : Theme.surface
    opacity: track.audible ? 1 : 0.55
    Behavior on color { ColorAnimation { duration: Theme.fast } }

    Rectangle {
        anchors.bottom: parent.bottom
        width: parent.width
        height: 1
        color: Theme.line
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        cursorShape: header.dragging ? Qt.ClosedHandCursor : Qt.ArrowCursor
        property real pressY
        onPressed: m => {
            header.app.selectTrack(header.track.id)
            if (m.button === Qt.RightButton) trackMenu.popup()
            pressY = mapToItem(header.parent, m.x, m.y).y
        }
        onPositionChanged: m => {
            if (!pressed || !(pressedButtons & Qt.LeftButton)) return
            const dy = mapToItem(header.parent, m.x, m.y).y - pressY
            if (!header.dragging && Math.abs(dy) < 6) return
            header.dragging = true
            header.dragY = dy
        }
        onReleased: {
            if (header.dragging && header.target !== header.index)
                header.app.moveTrack(header.track.id, header.target)
            header.dragging = false
            header.dragY = 0
        }
    }

    // Colour strip; click to choose another colour.
    Rectangle {
        id: strip
        x: header.track && header.track.group ? 22 : 6; y: 8
        width: 5
        height: parent.height - 16
        radius: 2.5
        color: header.tint
        MouseArea {
            anchors.fill: parent
            anchors.margins: -4
            cursorShape: Qt.PointingHandCursor
            onClicked: colorPopup.open()
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: header.track && header.track.group ? 36 : 20
        anchors.rightMargin: 8
        anchors.topMargin: 6
        anchors.bottomMargin: 5
        spacing: 4

        RowLayout {
            spacing: 6
            DText {
                text: header.index + 1
                font.family: Theme.mono
                font.pixelSize: Theme.textXs
                faint: true
                Layout.preferredWidth: 14
            }
            Icon {
                visible: !header.track.instrument
                glyph: Icons.waveform
                size: 15
                color: header.tint
            }
            Icon {
                visible: !!header.track.instrument
                glyph: Icons.pianoKeys
                size: 15
                color: header.tint
                DToolTip { visible: hoverInst.hovered; text: header.track.instrument ? "Open " + header.track.instrument.label + " · double-click the lane to add a MIDI region" : "" }
                HoverHandler { id: hoverInst; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: header.instrumentRequested() }
            }
            // Frozen (or freezing): click to unfreeze.
            AbstractButton {
                id: frozenBadge
                readonly property bool freezing: header.app.freezingTrack === header.track.id
                visible: !!header.track.frozen || freezing
                implicitWidth: 20; implicitHeight: 20
                hoverEnabled: true
                onClicked: if (!freezing) header.app.unfreezeTrack(header.track.id)
                background: Rectangle { radius: 10; color: Theme.tint("#7CC8FF", frozenBadge.hovered ? 0.35 : 0.2) }
                contentItem: DText {
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    text: frozenBadge.freezing ? Math.round(header.app.freezeProgress * 100) + "" : "❄"
                    font.pixelSize: frozenBadge.freezing ? 8 : 11
                    color: "#9AD6FF"
                }
                DToolTip { visible: frozenBadge.hovered; text: frozenBadge.freezing ? "Freezing…" : "Frozen: plays its rendered audio to save processing power. Click to unfreeze and edit it." }
            }
            Item {
                Layout.fillWidth: true
                implicitHeight: 22
                DText {
                    id: nameLabel
                    visible: !nameEdit.visible
                    anchors.fill: parent
                    text: header.track.name
                    strong: true
                    elide: Text.ElideRight
                    MouseArea {
                        anchors.fill: parent
                        onPressed: header.app.selectTrack(header.track.id)
                        onDoubleClicked: header.startRename()
                    }
                }
                TextInput {
                    id: nameEdit
                    visible: false
                    anchors.fill: parent
                    verticalAlignment: TextInput.AlignVCenter
                    font.family: Theme.font
                    font.pixelSize: Theme.textMd
                    font.weight: Font.DemiBold
                    color: Theme.text
                    selectionColor: Theme.accent
                    onAccepted: { header.app.renameTrack(header.track.id, text); visible = false }
                    onActiveFocusChanged: if (!activeFocus) visible = false
                    Keys.onEscapePressed: visible = false
                }
            }
        }

        // Automation, record, monitor, mute and solo; the volume in dB.
        RowLayout {
            spacing: 4
            MiniToggle {
                label: "A"
                on: !!header.track.automationView
                onColor: Theme.tint(header.tint, 0.9)
                tip: header.track.automationView ? "Hide automation" : "Show automation: draw volume, pan, sends or effect changes over time"
                onClicked: header.app.setAutomationView(header.track.id, header.track.automationView ? "" : "volume")
            }
            MiniToggle {
                visible: !header.track.instrument
                label: "R"
                on: header.track.armed
                onColor: Theme.record
                tip: header.track.armed ? "Armed for recording" : "Arm for recording"
                onClicked: header.app.toggleArm(header.track.id)
            }
            MiniToggle {
                visible: !header.track.instrument
                glyph: Icons.headphones
                on: header.track.monitor
                onColor: Theme.warning
                tip: "Hear the input while recording (use headphones to avoid feedback)"
                onClicked: header.app.toggleMonitor(header.track.id)
            }
            MiniToggle {
                label: "M"
                on: header.track.mute
                onColor: Theme.accent
                tip: "Mute"
                onClicked: header.app.toggleMute(header.track.id)
            }
            MiniToggle {
                label: "S"
                on: header.track.solo
                onColor: Theme.loopOn
                tip: "Solo"
                onClicked: header.app.toggleSolo(header.track.id)
            }
            Item { Layout.fillWidth: true }
            DText {
                Layout.preferredWidth: 46
                horizontalAlignment: Text.AlignRight
                text: fader.value <= -60 ? "-∞ dB" : (fader.value > 0 ? "+" : "") + fader.value.toFixed(1) + " dB"
                font.family: Theme.mono
                font.pixelSize: Theme.textXs
                dim: true
                MouseArea {
                    anchors.fill: parent
                    hoverEnabled: true
                    onDoubleClicked: header.app.setTrackVolume(header.track.id, 0, true)
                    DToolTip { visible: parent.containsMouse; text: "Double-click for 0 dB" }
                }
            }
        }

        RowLayout {
            spacing: 8
            // Volume: -60 dB (silent) to +6 dB; double-click for 0 dB.
            DSlider {
                id: fader
                Layout.fillWidth: true
                implicitHeight: 18
                from: -60; to: 6
                value: Auto.live(header.track, "volume", header.app.positionBeats, header.app.playing, header.track.volume)
                onMoved: header.app.setTrackVolume(header.track.id, value, false)
                onPressedChanged: if (!pressed) header.app.setTrackVolume(header.track.id, value, true)
            }
            PanKnob {
                value: Auto.live(header.track, "pan", header.app.positionBeats, header.app.playing, header.track.pan)
                tint: header.tint
                onMoved: (v, done) => header.app.setTrackPan(header.track.id, v, done)
            }
            // The track's level after its fader, left and right.
            Row {
                spacing: 2
                Repeater {
                    model: 2
                    delegate: Rectangle {
                        required property int index
                        readonly property real v: Math.max(0, Math.min(1, header.level[index] || 0))
                        width: 4; height: 26
                        radius: 1
                        color: Theme.sunken
                        Rectangle {
                            anchors.bottom: parent.bottom
                            width: parent.width
                            height: parent.height * parent.v
                            radius: 1
                            color: parent.v > 0.97 ? Theme.danger : parent.v > 0.85 ? Theme.warning : Theme.success
                        }
                    }
                }
            }
        }
    }

    // Live input level while armed.
    LevelMeter {
        visible: header.track.armed
        anchors.left: parent.left; anchors.leftMargin: 20
        anchors.bottom: parent.bottom; anchors.bottomMargin: 4
        meterWidth: header.width - 30
        leftLevel: header.app.inputLevelL
        rightLevel: header.app.inputLevelR
        spacing: 1
    }

    function startRename() {
        nameEdit.text = track.name
        nameEdit.visible = true
        nameEdit.forceActiveFocus()
        nameEdit.selectAll()
    }

    MenuPopup {
        id: trackMenu
        function popup() { x = 20; y = header.height - 6; open() }
        items: [
            { label: "Rename", action: () => header.startRename() },
            { label: "Colour…", action: () => colorPopup.open() },
            { label: header.track.frozen ? "Unfreeze Track" : "Freeze Track (save CPU)",
              enabled: header.app.freezingTrack < 0,
              action: () => header.track.frozen ? header.app.unfreezeTrack(header.track.id) : header.app.freezeTrack(header.track.id) },
            { separator: true },
            { label: "Delete Track", danger: true, action: () => header.app.removeTrack(header.track.id) }
        ]
    }

    Popup {
        id: colorPopup
        x: 14; y: 10
        padding: 10
        background: GlassPanel { backdrop: Theme.backdrop; radius: Theme.radiusLg; clarity: 0.3 }
        contentItem: Grid {
            columns: 4
            spacing: 8
            Repeater {
                model: Theme.trackColorNames
                delegate: AbstractButton {
                    id: swatch
                    required property string modelData
                    width: 24; height: 24
                    hoverEnabled: true
                    onClicked: {
                        // Close first: the change rebuilds the track list, which replaces this header.
                        const app = header.app, id = header.track.id, color = modelData
                        colorPopup.close()
                        app.setTrackColor(id, color)
                    }
                    background: Rectangle {
                        radius: 12
                        color: Theme.trackColor(swatch.modelData)
                        border.width: header.track.color === swatch.modelData ? 2 : 0
                        border.color: Theme.text
                        scale: swatch.hovered ? 1.12 : 1
                    }
                }
            }
        }
    }
}

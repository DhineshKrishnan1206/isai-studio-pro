import QtQuick
import QtQuick.Controls.Basic
import Isai

// Every keyboard shortcut, grouped, over a dimmed backdrop. Opens with ?
// (or the keyboard button); Esc or a click outside closes it.
Popup {
    id: overlay
    property Item backdrop: null
    readonly property string mod: Qt.platform.os === "osx" ? "⌘" : "Ctrl+"
    readonly property string alt: Qt.platform.os === "osx" ? "⌥" : "Alt+"

    readonly property var groups: [
        { title: "Transport", keys: [
            ["Space", "Play / stop"],
            ["0", "Stop · press again to go to the start"],
            ["Return", "Go to the start"],
            ["R", "Record"],
            ["C", "Cycle on / off"],
            [mod + "U", "Cycle the selected regions"],
            ["K", "Metronome"],
            ["T", "Tap tempo"]
        ] },
        { title: "Markers", keys: [
            ["M", "Add a marker at the playhead"],
            [alt + "→", "Next marker"],
            [alt + "←", "Previous marker"],
            ["Double-click a marker", "Rename it"]
        ] },
        { title: "Editing", keys: [
            [mod + "Z", "Undo"],
            ["⇧" + mod + "Z", "Redo"],
            [alt + mod + "Z", "Undo history"],
            [mod + "C / " + mod + "X / " + mod + "V", "Copy / cut / paste"],
            [mod + "D", "Duplicate"],
            [mod + "T", "Split at the playhead"],
            [mod + "J", "Join selected regions"],
            [mod + "A", "Select everything"],
            ["⌫", "Delete"],
            ["N", "Snap to grid"],
            [alt + "-click", "Split a region where you click"]
        ] },
        { title: "Windows", keys: [
            ["F5", "Playlist (just the tracks)"],
            ["F7 or P", "Piano roll"],
            ["F9 or X", "Mixer"],
            ["B", "Sound browser"],
            ["I", "Inspector"],
            ["G", "Tempo & signature lanes"],
            [mod + "K", "Search every command"],
            ["?", "This list"]
        ] },
        { title: "Piano roll", keys: [
            ["Q / ⇧Q", "Quantize starts / starts and ends"],
            ["← → ↑ ↓", "Nudge notes · ⇧↑↓ an octave"],
            ["Right-drag", "Erase notes"],
            ["⇧-drag", "Select a box of notes"],
            ["Esc", "Clear the selection"]
        ] },
        { title: "View & files", keys: [
            [mod + "= / " + mod + "−", "Zoom in / out"],
            ["Z", "Fit the whole song"],
            [mod + " + scroll", "Zoom at the mouse"],
            [mod + "S", "Save"],
            [mod + "E", "Export"],
            [mod + "I", "Import audio"],
            [alt + mod + "N", "New track"]
        ] }
    ]

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(860, (parent ? parent.width : 900) - 48)
    height: Math.min(columns.implicitHeight + 76, (parent ? parent.height : 700) - 48)
    padding: 22
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    Overlay.modal: Rectangle { color: Qt.rgba(0, 0, 0, 0.45) }
    background: GlassPanel { backdrop: overlay.backdrop; radius: Theme.radiusXl; clarity: 0.25 }
    enter: Transition { NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.fast } }
    exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: Theme.fast } }

    contentItem: Item {
        Row {
            id: head
            spacing: 10
            Icon { glyph: Icons.keyboard; size: 18; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter }
            DText { text: "Keyboard shortcuts"; strong: true; font.pixelSize: Theme.textLg; anchors.verticalCenter: parent.verticalCenter }
        }
        DText {
            anchors.right: parent.right
            anchors.verticalCenter: head.verticalCenter
            text: "Esc to close"
            faint: true
            font.pixelSize: Theme.textXs
        }
        Flickable {
            y: head.height + 18
            width: parent.width
            height: parent.height - y
            contentHeight: columns.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            Flow {
                id: columns
                width: parent.width
                spacing: 22
                Repeater {
                    model: overlay.groups
                    delegate: Column {
                        id: group
                        required property var modelData
                        width: (columns.width - columns.spacing * 2) / 3
                        spacing: 4
                        DText {
                            text: group.modelData.title.toUpperCase()
                            faint: true
                            font.pixelSize: Theme.textXs
                            font.weight: Font.DemiBold
                            font.letterSpacing: 1
                            bottomPadding: 4
                        }
                        Repeater {
                            model: group.modelData.keys
                            delegate: Item {
                                required property var modelData
                                width: group.width
                                height: Math.max(keyCap.height, what.implicitHeight) + 4
                                Rectangle {
                                    id: keyCap
                                    width: Math.min(group.width * 0.45, keyText.implicitWidth + 12)
                                    height: 22
                                    radius: Theme.radiusSm
                                    color: Theme.tint(Theme.sunken, 0.8)
                                    border.color: Theme.line
                                    DText {
                                        id: keyText
                                        anchors.centerIn: parent
                                        width: Math.min(implicitWidth, parent.width - 8)
                                        elide: Text.ElideRight
                                        text: modelData[0]
                                        font.family: Theme.mono
                                        font.pixelSize: Theme.textXs
                                    }
                                }
                                DText {
                                    id: what
                                    x: keyCap.width + 10
                                    width: parent.width - x
                                    anchors.verticalCenter: keyCap.verticalCenter
                                    text: modelData[1]
                                    wrapMode: Text.WordWrap
                                    dim: true
                                    font.pixelSize: Theme.textSm
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

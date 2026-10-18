import QtQuick
import Isai

// A channel-strip fader with a stereo level meter beside it.
//
// The fader has more travel near 0 dB, where small changes matter, like a
// mixing console. Drag the cap (⇧ for fine moves), scroll over it, or
// double-click for 0 dB. `moved(db, done)`: done is true when the move ends.
Item {
    id: fader
    property real db: 0
    property real minDb: -60
    property real maxDb: 6
    /// 0..1 on a -60..0 dB scale, like LevelMeter.
    property real leftLevel: 0
    property real rightLevel: 0
    property color tint: Theme.accent
    signal moved(real db, bool done)
    /// What the fader shows: the dragged value while dragging (changes are
    /// only published when the drag ends), otherwise `db`.
    property real liveDb: 0
    readonly property real shownDb: drag.pressed ? liveDb : db

    implicitWidth: 40
    implicitHeight: 180

    readonly property int capHeight: 22
    readonly property real travel: height - capHeight

    function posOf(d) { return d <= minDb ? 0 : Math.pow((Math.min(d, maxDb) - minDb) / (maxDb - minDb), 2) }
    function dbOf(p) { return p <= 0.0005 ? minDb : minDb + Math.sqrt(Math.min(1, p)) * (maxDb - minDb) }
    function yOf(d) { return capHeight / 2 + (1 - posOf(d)) * travel }
    function emit(d, done) {
        const v = Math.round(Math.max(minDb, Math.min(maxDb, d)) * 10) / 10
        liveDb = v
        moved(v, done)
    }

    // Scale marks
    Repeater {
        model: [6, 0, -6, -12, -24, -48]
        delegate: Rectangle {
            required property real modelData
            visible: modelData <= fader.maxDb
            x: 2
            y: fader.yOf(modelData)
            width: modelData === 0 ? 10 : 6
            height: 1
            color: modelData === 0 ? Theme.textDim : Theme.lineStrong
        }
    }

    // Groove
    Rectangle {
        x: 13
        y: fader.capHeight / 2
        width: 4
        height: fader.travel
        radius: 2
        color: Theme.sunken
        border.color: Theme.line
        // Filled up to the cap, in the track colour.
        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: fader.posOf(fader.shownDb) * parent.height
            radius: 2
            color: Theme.tint(fader.tint, 0.7)
        }
    }

    // Meters
    Repeater {
        model: [fader.leftLevel, fader.rightLevel]
        delegate: Rectangle {
            required property real modelData
            required property int index
            x: 28 + index * 5
            y: fader.capHeight / 2
            width: 3
            height: fader.travel
            radius: 1.5
            color: Theme.sunken
            Item {
                anchors.bottom: parent.bottom
                width: parent.width
                height: parent.height * Math.max(0, Math.min(1, parent.modelData))
                clip: true
                Rectangle {
                    anchors.bottom: parent.bottom
                    width: parent.width
                    height: fader.travel
                    radius: 1.5
                    gradient: Gradient {
                        GradientStop { position: 0.0; color: Theme.danger }
                        GradientStop { position: 0.14; color: Theme.warning }
                        GradientStop { position: 0.28; color: Theme.success }
                        GradientStop { position: 1.0; color: Theme.success }
                    }
                }
            }
        }
    }

    // Cap
    Rectangle {
        id: cap
        x: 3
        y: fader.yOf(fader.shownDb) - height / 2
        width: 24
        height: fader.capHeight
        radius: 4
        color: drag.pressed ? Theme.hover : Theme.raised
        border.color: drag.containsMouse || drag.pressed ? Theme.textFaint : Theme.lineStrong
        gradient: Gradient {
            GradientStop { position: 0; color: Qt.lighter(cap.color, 1.18) }
            GradientStop { position: 1; color: cap.color }
        }
        Rectangle { anchors.centerIn: parent; width: 16; height: 2; radius: 1; color: fader.tint }
    }

    MouseArea {
        id: drag
        x: 0
        width: 30
        height: parent.height
        hoverEnabled: true
        preventStealing: true
        cursorShape: Qt.SizeVerCursor
        property real startY
        property real startPos
        onPressed: m => {
            // Clicking the groove away from the cap jumps there; grabbing the cap keeps its offset.
            fader.liveDb = fader.db
            if (Math.abs(m.y - fader.yOf(fader.db)) > fader.capHeight / 2)
                fader.emit(fader.dbOf(1 - (m.y - fader.capHeight / 2) / fader.travel), false)
            startY = m.y
            startPos = fader.posOf(fader.liveDb)
        }
        onPositionChanged: m => {
            if (!pressed) return
            const scale = (m.modifiers & Qt.ShiftModifier) ? 0.2 : 1
            fader.emit(fader.dbOf(startPos + (startY - m.y) / fader.travel * scale), false)
        }
        onReleased: fader.emit(fader.liveDb, true)
        onDoubleClicked: fader.emit(0, true)
        onWheel: w => {
            const step = (w.angleDelta.y !== 0 ? w.angleDelta.y / 120 : w.pixelDelta.y / 30) * 0.5
            fader.emit(fader.db + step, true)
        }
        DToolTip {
            visible: drag.containsMouse && !drag.pressed
            text: "Volume: how loud this track is. Double-click for 0 dB · hold ⇧ to fine-tune"
        }
    }
}

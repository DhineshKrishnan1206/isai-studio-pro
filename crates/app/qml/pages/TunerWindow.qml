import QtCore
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Shapes
import Isai

// A tuner for guitars, basses and voices: it listens to the recording input
// while it's open. The needle shows how far the note is from in tune (in
// cents); green means in tune. Guitar and Bass modes also name the nearest
// open string. Drag the title to move it.
Item {
    id: tuner
    property var app
    property bool shown: false
    property Item backdrop: null
    signal closeRequested()

    width: 340
    height: 318
    visible: opacity > 0
    opacity: shown ? 1 : 0
    Behavior on opacity { NumberAnimation { duration: Theme.fast } }
    z: 160

    onShownChanged: {
        app.setTuner(shown)
        if (shown && parent) { x = parent.width - width - 24; y = 60 }
    }

    Settings {
        id: prefs
        category: "tuner"
        property real a4: 440
        property string mode: "chromatic"
    }

    // ------------------------------------------------------------ reading
    property var reading: ({})
    property real cents: 0
    property string note: ""
    property int octave: 0
    property real hz: 0
    property int midi: -1
    property real heardAt: 0
    property real now: 0
    readonly property bool hasPitch: note !== "" && now - heardAt < 900
    readonly property real accuracy: Math.abs(cents)
    readonly property color needleColor: !hasPitch ? Theme.textFaint
                                         : accuracy < 5 ? Theme.success : accuracy < 15 ? Theme.warning : Theme.danger

    readonly property var strings: ({
        guitar: [["E", 40], ["A", 45], ["D", 50], ["G", 55], ["B", 59], ["E", 64]],
        bass: [["E", 28], ["A", 33], ["D", 38], ["G", 43]]
    })
    /// The open string nearest the note being played (Guitar and Bass modes).
    readonly property int nearestString: {
        const list = strings[prefs.mode]
        if (!list || midi < 0) return -1
        let best = 0
        list.forEach((s, i) => { if (Math.abs(s[1] - midi) < Math.abs(list[best][1] - midi)) best = i })
        return best
    }

    Timer {
        running: tuner.shown
        interval: 50
        repeat: true
        onTriggered: {
            let r
            try { r = JSON.parse(tuner.app.tunerReading(prefs.a4) || "{}") } catch (e) { return }
            tuner.reading = r
            tuner.now = Date.now()
            if (r.hz !== undefined && r.clarity > 0.75) {
                // A new note jumps straight there; the same note glides, so the needle doesn't jitter.
                tuner.cents = r.midi === tuner.midi ? tuner.cents + (r.cents - tuner.cents) * 0.35 : r.cents
                tuner.note = r.note
                tuner.octave = r.octave
                tuner.hz = r.hz
                tuner.midi = r.midi
                tuner.heardAt = tuner.now
            }
        }
    }

    // ------------------------------------------------------------ body
    GlassPanel {
        anchors.fill: parent
        backdrop: tuner.backdrop
        radius: Theme.radiusXl
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons; onWheel: w => w.accepted = true }
    }

    Item {
        id: titleBar
        width: parent.width
        height: 40
        DragHandler { target: tuner }
        Row {
            x: 16
            anchors.verticalCenter: parent.verticalCenter
            spacing: 8
            Icon { glyph: Icons.gauge; size: 15; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter }
            DText { text: "Tuner"; strong: true; anchors.verticalCenter: parent.verticalCenter }
            DText {
                anchors.verticalCenter: parent.verticalCenter
                text: tuner.app.inputOk ? tuner.app.inputName : ""
                faint: true
                font.pixelSize: Theme.textXs
                width: 160
                elide: Text.ElideRight
            }
        }
        IconButton {
            anchors.right: parent.right; anchors.rightMargin: 8
            anchors.verticalCenter: parent.verticalCenter
            implicitWidth: 26; implicitHeight: 26; iconSize: 13
            glyph: Icons.x
            tip: "Close the tuner"
            onClicked: tuner.closeRequested()
        }
    }

    Segmented {
        x: (parent.width - width) / 2
        y: 44
        implicitHeight: 26
        minItemWidth: 76
        options: [
            { value: "chromatic", label: "Chromatic" },
            { value: "guitar", label: "Guitar" },
            { value: "bass", label: "Bass" }
        ]
        value: prefs.mode
        onActivated: v => prefs.mode = v
    }

    // Needle scale: −50 … +50 cents across an arc.
    Item {
        id: dial
        x: 20; y: 80
        width: parent.width - 40
        height: 132
        readonly property real cx: width / 2
        readonly property real cy: height - 4
        readonly property real r: Math.min(width / 2 - 6, height - 10)
        Shape {
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                strokeColor: Theme.line
                strokeWidth: 6
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap
                PathAngleArc { centerX: dial.cx; centerY: dial.cy; radiusX: dial.r; radiusY: dial.r; startAngle: 210; sweepAngle: 120 }
            }
            // The in-tune zone (±5 cents).
            ShapePath {
                strokeColor: Theme.tint(Theme.success, 0.8)
                strokeWidth: 6
                fillColor: "transparent"
                PathAngleArc { centerX: dial.cx; centerY: dial.cy; radiusX: dial.r; radiusY: dial.r; startAngle: 264; sweepAngle: 12 }
            }
        }
        // Ticks every 10 cents.
        Repeater {
            model: 11
            delegate: Rectangle {
                id: tick
                required property int index
                readonly property real angle: (index - 5) * 12
                x: dial.cx - width / 2
                y: dial.cy - dial.r - 14
                width: index === 5 ? 2 : 1
                height: index % 5 === 0 ? 10 : 6
                color: Theme.textFaint
                transform: Rotation { origin.x: tick.width / 2; origin.y: dial.r + 14; angle: tick.angle }
            }
        }
        DText { x: 4; y: dial.cy - 28; text: "♭"; faint: true; font.pixelSize: Theme.textLg }
        DText { x: dial.width - 14; y: dial.cy - 28; text: "♯"; faint: true; font.pixelSize: Theme.textLg }
        // The needle.
        Rectangle {
            x: dial.cx - width / 2
            y: dial.cy - dial.r + 2
            width: 3
            height: dial.r
            radius: 1.5
            color: tuner.needleColor
            antialiasing: true
            transform: Rotation {
                origin.x: 1.5; origin.y: dial.r
                angle: tuner.hasPitch ? Math.max(-50, Math.min(50, tuner.cents)) * 1.2 : 0
                Behavior on angle { NumberAnimation { duration: Theme.fast } }
            }
        }
        Rectangle { x: dial.cx - 6; y: dial.cy - 6; width: 12; height: 12; radius: 6; color: tuner.needleColor }
    }

    // Note name and how far off it is.
    Column {
        anchors.horizontalCenter: parent.horizontalCenter
        y: 150
        spacing: 0
        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 2
            DText {
                text: tuner.hasPitch ? tuner.note : "–"
                font.pixelSize: 44
                font.weight: Font.Bold
                color: tuner.hasPitch ? tuner.needleColor : Theme.textFaint
                Behavior on color { ColorAnimation { duration: Theme.fast } }
            }
            DText {
                visible: tuner.hasPitch
                anchors.bottom: parent.bottom; anchors.bottomMargin: 8
                text: tuner.octave
                font.pixelSize: Theme.textLg
                dim: true
            }
        }
    }

    // Guitar / bass strings, nearest one lit.
    Row {
        visible: prefs.mode !== "chromatic"
        anchors.horizontalCenter: parent.horizontalCenter
        y: 222
        spacing: 6
        Repeater {
            model: tuner.strings[prefs.mode] || []
            delegate: Rectangle {
                required property var modelData
                required property int index
                readonly property bool near: tuner.hasPitch && tuner.nearestString === index
                width: 30; height: 24; radius: Theme.radiusSm
                color: near ? Theme.tint(tuner.needleColor, 0.25) : Theme.tint(Theme.sunken, 0.6)
                border.color: near ? tuner.needleColor : Theme.line
                DText { anchors.centerIn: parent; text: parent.modelData[0]; strong: parent.near; font.pixelSize: Theme.textSm }
            }
        }
    }

    Row {
        anchors.horizontalCenter: parent.horizontalCenter
        y: prefs.mode === "chromatic" ? 226 : 254
        spacing: 14
        DText {
            text: tuner.hasPitch ? (tuner.cents > 0 ? "+" : "") + Math.round(tuner.cents) + " cents" : "Play one note"
            font.family: Theme.mono
            font.pixelSize: Theme.textSm
            dim: true
        }
        DText {
            visible: tuner.hasPitch
            text: tuner.hz.toFixed(1) + " Hz"
            font.family: Theme.mono
            font.pixelSize: Theme.textSm
            faint: true
        }
    }

    // Bottom: input level and the reference pitch.
    Item {
        x: 16
        width: parent.width - 32
        y: parent.height - 34
        height: 24
        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: 90; height: 4; radius: 2
            color: Theme.line
            Rectangle {
                width: parent.width * Math.max(0, Math.min(1, tuner.reading.level || 0))
                height: parent.height; radius: 2
                color: Theme.success
            }
        }
        DText {
            x: 98
            anchors.verticalCenter: parent.verticalCenter
            visible: !tuner.app.inputOk
            text: "No input: pick one in Settings"
            font.pixelSize: Theme.textXs
            color: Theme.warning
        }
        Row {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            spacing: 4
            IconButton { implicitWidth: 22; implicitHeight: 22; iconSize: 10; glyph: Icons.caretDown; tip: "Lower the reference pitch"; onClicked: prefs.a4 = Math.max(400, prefs.a4 - 1) }
            DText {
                anchors.verticalCenter: parent.verticalCenter
                text: "A = " + prefs.a4 + " Hz"
                font.family: Theme.mono
                font.pixelSize: Theme.textXs
                dim: true
                MouseArea { anchors.fill: parent; onDoubleClicked: prefs.a4 = 440 }
            }
            IconButton { implicitWidth: 22; implicitHeight: 22; iconSize: 10; glyph: Icons.caretUp; tip: "Raise the reference pitch (double-click the number for 440)"; onClicked: prefs.a4 = Math.min(480, prefs.a4 + 1) }
        }
    }
}

import QtQuick
import Isai

// A video clip on a video track: its picture (the frame at the playhead) and
// its name. Drag it along the track to move it; double-click to open the panel.
Rectangle {
    id: block
    /// The clip (an entry of the project's videos).
    required property var clipData
    required property var app
    /// Pixels per beat on the arrangement.
    required property real pxPerBeat
    signal opened()

    /// How far the clip is being dragged, in pixels (applied when released).
    property real dragX: 0
    readonly property bool dragging: drag.pressed

    x: clipData.beat * pxPerBeat + dragX
    y: 4
    width: Math.max(8, clipData.beats * pxPerBeat)
    height: parent.height - 8
    radius: 4
    clip: true
    color: Theme.raised
    border.color: dragging ? Theme.accent : Theme.lineStrong
    z: dragging ? 2 : 0

    VideoPreview {
        anchors.fill: parent
        anchors.margins: 1
        path: block.clipData.path
        time: block.app.position - block.clipData.start
        hold: block.app.playing
        visible: block.width > 24
    }

    // The clip's name, over the top of its picture.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 18
        color: Qt.rgba(0, 0, 0, 0.55)
        Text {
            anchors.fill: parent
            anchors.leftMargin: 6
            anchors.rightMargin: 6
            verticalAlignment: Text.AlignVCenter
            text: block.clipData.name
            color: "#FFFFFF"
            font.pixelSize: 11
            elide: Text.ElideRight
        }
    }

    MouseArea {
        id: drag
        anchors.fill: parent
        cursorShape: Qt.SizeHorCursor
        property real pressX: 0
        onPressed: (m) => { pressX = m.x; block.dragX = 0 }
        onPositionChanged: (m) => { if (pressed) block.dragX = m.x - pressX }
        onReleased: {
            const beats = block.clipData.beat + block.dragX / block.pxPerBeat
            block.dragX = 0
            if (Math.abs(beats - block.clipData.beat) > 0.001) block.app.moveVideo(block.clipData.id, block.app.snapBeats(beats))
        }
        onDoubleClicked: block.opened()
    }
}

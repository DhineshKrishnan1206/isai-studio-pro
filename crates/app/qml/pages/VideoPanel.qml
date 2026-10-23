import QtQuick
import QtQuick.Controls.Basic
import QtMultimedia
import Isai

// The video panel: plays the video under the playhead, in step with the timeline,
// for scoring to picture. It sits inside the window, docked to one side of the
// arrangement. Its sound is muted by default, since the mix comes from the tracks.
Item {
    id: panel
    property var app
    /// Which side of the arrangement it's docked to: "left", "right", "top" or "bottom".
    property string dock: "right"
    /// Its size across the dock (width for left/right, height for top/bottom), in pixels.
    property real size: 380
    signal closeRequested()
    signal dockRequested(string side)
    signal resizeRequested(real px)

    readonly property var clips: {
        try { return JSON.parse(app.videosJson || "[]") } catch (e) { return [] }
    }
    /// The clip under the playhead: the last one that has started, else the first.
    readonly property var clip: {
        let found = null
        for (const c of clips) if (c.start <= app.position) found = c
        return found || (clips.length ? clips[0] : null)
    }
    readonly property bool before: !!clip && app.position < clip.start
    /// Seconds into the clip at the playhead.
    readonly property real clipTime: clip ? Math.max(0, app.position - clip.start) : 0

    property bool muted: true
    property real volume: 1
    property bool fit: true
    property string problem: ""

    function fileUrl(path) {
        const p = path.replace(/\\/g, "/")
        const encoded = encodeURI(p).replace(/#/g, "%23").replace(/\?/g, "%3F")
        return "file://" + (p.startsWith("/") ? "" : "/") + encoded
    }

    function clock(secs) {
        const s = Math.max(0, secs)
        const m = Math.floor(s / 60)
        const rest = (s - m * 60).toFixed(1).padStart(4, "0")
        return m + ":" + rest
    }

    // Keeps the clip in step with the timeline.
    function sync(force) {
        if (!clip) return
        if (before) { player.pause(); return }
        const want = clipTime * 1000
        if (force || Math.abs(player.position - want) > 150) player.position = want
        if (app.playing && player.playbackState !== MediaPlayer.PlayingState) player.play()
        else if (!app.playing && player.playbackState === MediaPlayer.PlayingState) player.pause()
    }

    Connections {
        target: panel.app
        function onPlayingChanged() { panel.sync(true) }
        function onPositionChanged() { panel.sync(false) }
    }

    MediaPlayer {
        id: player
        source: panel.clip ? panel.fileUrl(panel.clip.path) : ""
        videoOutput: picture
        audioOutput: AudioOutput { muted: panel.muted; volume: panel.volume }
        onMediaStatusChanged: if (mediaStatus === MediaPlayer.LoadedMedia) panel.sync(true)
        onErrorOccurred: (error, message) => panel.problem = message
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.panel
    }

    // Edge to drag to change the panel's size.
    MouseArea {
        id: resizer
        readonly property bool across: panel.dock === "left" || panel.dock === "right"
        anchors.left: panel.dock === "right" ? parent.left : panel.dock === "left" ? undefined : parent.left
        anchors.right: panel.dock === "left" ? parent.right : panel.dock === "right" ? undefined : parent.right
        anchors.top: panel.dock === "bottom" ? parent.top : panel.dock === "top" ? undefined : parent.top
        anchors.bottom: panel.dock === "top" ? parent.bottom : panel.dock === "bottom" ? undefined : parent.bottom
        width: across ? 5 : parent.width
        height: across ? parent.height : 5
        cursorShape: across ? Qt.SplitHCursor : Qt.SplitVCursor
        z: 3
        onPositionChanged: (m) => {
            if (!pressed) return
            const p = mapToItem(panel.parent, m.x, m.y)
            if (panel.dock === "left") panel.resizeRequested(p.x)
            else if (panel.dock === "right") panel.resizeRequested(panel.parent.width - p.x)
            else if (panel.dock === "top") panel.resizeRequested(p.y)
            else panel.resizeRequested(panel.parent.height - p.y)
        }
    }

    Column {
        anchors.fill: parent
        spacing: 0

        Item {
            id: bar
            width: parent.width
            height: 36

            Text {
                anchors.left: parent.left
                anchors.leftMargin: 12
                anchors.right: docks.left
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                text: panel.clip ? panel.clip.name : "Video"
                color: Theme.text
                font.pixelSize: 13
                elide: Text.ElideRight
            }

            Row {
                id: docks
                anchors.right: parent.right
                anchors.rightMargin: 6
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2

                IconButton { label: "Left"; tip: "Dock to the left"; active: panel.dock === "left"; onClicked: panel.dockRequested("left") }
                IconButton { label: "Right"; tip: "Dock to the right"; active: panel.dock === "right"; onClicked: panel.dockRequested("right") }
                IconButton { label: "Top"; tip: "Dock above the arrangement"; active: panel.dock === "top"; onClicked: panel.dockRequested("top") }
                IconButton { label: "Bottom"; tip: "Dock below the arrangement"; active: panel.dock === "bottom"; onClicked: panel.dockRequested("bottom") }
                IconButton { glyph: Icons.x; tip: "Close the video panel"; onClicked: panel.closeRequested() }
            }
        }

        Rectangle {
            id: screen
            width: parent.width
            height: parent.height - bar.height - controls.height
            color: "#000000"

            VideoOutput {
                id: picture
                anchors.fill: parent
                fillMode: panel.fit ? VideoOutput.PreserveAspectFit : VideoOutput.PreserveAspectCrop
            }

            Text {
                anchors.centerIn: parent
                visible: !panel.clip || panel.before
                text: !panel.clip ? "Add a video with the Video button"
                    : "Starts at " + panel.clock(panel.clip.start) + " on the timeline"
                color: Theme.textDim
                font.pixelSize: 13
            }

            Text {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.margins: 10
                visible: panel.problem !== ""
                text: "Can't play this video: " + panel.problem
                color: Theme.danger
                font.pixelSize: 12
                wrapMode: Text.WordWrap
            }
        }

        Item {
            id: controls
            width: parent.width
            height: 46

            Row {
                anchors.left: parent.left
                anchors.leftMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4

                IconButton { glyph: Icons.skipBack; tip: "Back to the start of the clip"; enabled: !!panel.clip; onClicked: panel.app.seekBeats(panel.clip.beat) }
                IconButton { glyph: panel.app.playing ? Icons.stop : Icons.play; tip: panel.app.playing ? "Stop" : "Play"; onClicked: panel.app.togglePlay() }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    width: 110
                    text: panel.clock(panel.clipTime)
                    color: Theme.text
                    font.family: "Geist Mono"
                    font.pixelSize: 13
                }
            }

            Row {
                anchors.right: parent.right
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                spacing: 6

                Slider {
                    anchors.verticalCenter: parent.verticalCenter
                    width: 90
                    from: 0
                    to: 1
                    value: panel.volume
                    enabled: !panel.muted
                    onMoved: panel.volume = value
                }
                IconButton {
                    glyph: panel.muted ? Icons.speakerX : Icons.speakerHigh
                    tip: panel.muted ? "Hear the clip's sound" : "Mute the clip's sound"
                    active: !panel.muted
                    onClicked: panel.muted = !panel.muted
                }
                IconButton { label: panel.fit ? "Fit" : "Fill"; tip: panel.fit ? "Show the whole picture" : "Fill the panel, cropping the edges"; onClicked: panel.fit = !panel.fit }
            }
        }
    }
}

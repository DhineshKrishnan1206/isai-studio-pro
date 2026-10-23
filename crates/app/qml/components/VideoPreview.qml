import QtQuick
import QtMultimedia
import Isai

// A still frame of a video, shown inside an arrangement clip. It shows the frame
// at `time` (seconds into the video) and follows it while the song is stopped.
// While the song plays it holds its frame, so decoding doesn't compete with playback.
Item {
    id: preview
    /// The video file's path on disk.
    property string path: ""
    /// Seconds into the video that the frame shows.
    property real time: 0
    /// Holds the current frame instead of following `time`.
    property bool hold: false

    clip: true

    readonly property url url: path === "" ? "" : "file://" + (path.startsWith("/") ? "" : "/")
        + encodeURI(path.replace(/\\/g, "/")).replace(/#/g, "%23").replace(/\?/g, "%3F")

    // Shows the frame at `time`. Always seeks when `force` is set (a freshly loaded
    // clip has no frame yet). A paused player often doesn't draw a seek, so it
    // plays for a moment and pauses again.
    function sync(force) {
        if (hold || path === "" || player.mediaStatus !== MediaPlayer.LoadedMedia) return
        const want = Math.max(0, time) * 1000
        if (!force && Math.abs(player.position - want) <= 120) return
        player.position = want
        player.play()
        showTimer.restart()
    }
    Timer {
        id: showTimer
        interval: 120
        onTriggered: if (!preview.hold) player.pause()
    }
    onTimeChanged: sync(false)
    onHoldChanged: sync(true)

    MediaPlayer {
        id: player
        source: preview.url
        videoOutput: picture
        onMediaStatusChanged: if (mediaStatus === MediaPlayer.LoadedMedia) preview.sync(true)
    }

    VideoOutput {
        id: picture
        anchors.fill: parent
        fillMode: VideoOutput.PreserveAspectCrop
    }
}

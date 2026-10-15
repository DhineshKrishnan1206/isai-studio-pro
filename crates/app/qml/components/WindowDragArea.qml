import QtQuick

// Empty space that moves the window, like a second title bar.
//
// Declare it FIRST inside its parent, so every control declared after it sits
// on top and keeps its own clicks and drags. Only presses on empty space reach
// this area. Double-click maximises or restores, like a title bar.
//
// The move is handed to the system (startSystemMove), so it behaves like a real
// title bar: snapping, other displays, and other desktops (macOS Spaces,
// Mission Control). If the system can't start one, the window follows the
// mouse instead.
MouseArea {
    id: area
    acceptedButtons: Qt.LeftButton

    /// The window is following the mouse ourselves (no system move).
    property bool manual: false
    property point pressGlobal
    property point windowStart
    property real lastPress: 0
    property point lastPressAt

    onPressed: m => {
        const w = Window.window
        const at = mapToGlobal(m.x, m.y)
        // A system move swallows the release, so spot double-clicks here.
        const now = Date.now()
        const twice = now - lastPress < 400 && Math.abs(at.x - lastPressAt.x) + Math.abs(at.y - lastPressAt.y) < 6
        lastPress = twice ? 0 : now
        lastPressAt = at
        if (twice) {
            w.visibility === Window.Maximized ? w.showNormal() : w.showMaximized()
            manual = false
            return
        }
        manual = !w.startSystemMove()
        pressGlobal = at
        windowStart = Qt.point(w.x, w.y)
    }
    onPositionChanged: m => {
        if (!manual || !pressed) return
        const w = Window.window
        if (w.visibility === Window.FullScreen) return
        const now = mapToGlobal(m.x, m.y)
        if (w.visibility === Window.Maximized) {
            // Restore to its normal size, keeping the same spot of the bar under the cursor.
            if (Math.abs(now.x - pressGlobal.x) + Math.abs(now.y - pressGlobal.y) < 6) return
            const share = (pressGlobal.x - w.x) / Math.max(1, w.width)
            w.showNormal()
            windowStart = Qt.point(now.x - share * w.width, now.y - (pressGlobal.y - w.y))
            pressGlobal = now
            return
        }
        // The cursor's screen position, measured fresh each time, so moving
        // the window under it doesn't skew the result.
        w.x = windowStart.x + now.x - pressGlobal.x
        w.y = windowStart.y + now.y - pressGlobal.y
    }
    onReleased: manual = false
    onCanceled: manual = false
}

import QtQuick
import QtQuick.Controls.Basic
import Isai

// The undo history: every step, oldest at the top. Click a step to go back
// (or forward) to just after it; the steps after the current one stay
// available to redo until something new is edited.
Popup {
    id: pop
    property var app

    readonly property var steps: {
        try { return JSON.parse(app.historyJson || "{}") } catch (e) { return {} }
    }
    readonly property var undo: steps.undo || []
    readonly property var redo: steps.redo || []
    /// Rows: the starting point, the steps done, then the steps that can be redone.
    readonly property var rows: [{ label: "Start", done: true }]
        .concat(undo.map(l => ({ label: l, done: true })))
        .concat(redo.map(l => ({ label: l, done: false })))
    readonly property int current: undo.length

    padding: 6
    width: 260
    modal: false
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    background: GlassPanel { backdrop: Theme.backdrop; radius: Theme.radiusLg; clarity: 0.3 }
    enter: Transition { NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.fast } }
    exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: Theme.fast } }
    onAboutToShow: Qt.callLater(() => list.positionViewAtIndex(pop.current, ListView.Center))

    contentItem: Column {
        spacing: 4
        DText {
            leftPadding: 8; topPadding: 4
            text: "HISTORY"
            faint: true
            font.pixelSize: Theme.textXs
            font.weight: Font.DemiBold
            font.letterSpacing: 1
        }
        ListView {
            id: list
            width: pop.availableWidth
            height: Math.min(360, contentHeight)
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            model: pop.rows
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
            delegate: AbstractButton {
                id: row
                required property var modelData
                required property int index
                readonly property bool here: index === pop.current
                width: list.width
                height: 28
                hoverEnabled: true
                onClicked: pop.app.jumpHistory(index - pop.current)
                background: Rectangle {
                    radius: Theme.radiusSm
                    color: row.here ? Theme.accentSoft : row.hovered ? Theme.hover : "transparent"
                    border.color: row.here ? Theme.tint(Theme.accent, 0.5) : "transparent"
                }
                contentItem: Row {
                    spacing: 8
                    leftPadding: 8
                    Rectangle {
                        anchors.verticalCenter: parent.verticalCenter
                        width: 6; height: 6; radius: 3
                        color: row.here ? Theme.accent : row.modelData.done ? Theme.textDim : "transparent"
                        border.color: Theme.textFaint
                        border.width: row.modelData.done ? 0 : 1
                    }
                    DText {
                        anchors.verticalCenter: parent.verticalCenter
                        width: list.width - 40
                        elide: Text.ElideRight
                        text: row.modelData.label
                        color: row.modelData.done ? Theme.text : Theme.textFaint
                        font.pixelSize: Theme.textSm
                    }
                }
            }
        }
        DText {
            visible: pop.undo.length === 0 && pop.redo.length === 0
            leftPadding: 8; bottomPadding: 6
            width: pop.availableWidth
            wrapMode: Text.WordWrap
            text: "Your edits show up here. Click one to go back to it."
            faint: true
            font.pixelSize: Theme.textSm
        }
    }
}

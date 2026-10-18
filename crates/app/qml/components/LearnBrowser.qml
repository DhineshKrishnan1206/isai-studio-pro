import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// The lesson list: every lesson, grouped by what a beginner is looking for.
// Click a lesson to read it. Esc or a click outside closes the list.
Popup {
    id: browser
    /// What the glass panel blurs (usually the studio page behind it).
    property Item backdrop: null

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(560, (parent ? parent.width : 700) - 48)
    height: Math.min(600, (parent ? parent.height : 700) - 48)
    padding: 22
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    Overlay.modal: Rectangle { color: Qt.rgba(0, 0, 0, 0.4) }
    background: GlassPanel { backdrop: browser.backdrop; radius: Theme.radiusXl; clarity: 0.25 }
    enter: Transition { NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.fast } }
    exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: Theme.fast } }

    // The lesson opened from the list (one card, reused).
    LearnCard { id: card; backdrop: browser.backdrop }

    function openLesson(id) {
        browser.close()
        card.topic = id
        card.open()
    }

    contentItem: ColumnLayout {
        spacing: 12

        RowLayout {
            Layout.fillWidth: true
            spacing: 10
            Icon { glyph: Icons.info; size: 18; color: Theme.accent }
            DText {
                Layout.fillWidth: true
                text: "Learn"
                strong: true
                font.pixelSize: Theme.textLg
            }
            DText { text: "Click a lesson · Esc to close"; faint: true; font.pixelSize: Theme.textXs }
        }

        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: width
            contentHeight: groupColumn.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

            ColumnLayout {
                id: groupColumn
                width: parent.width - 12
                spacing: 16

                Repeater {
                    model: Learn.groups
                    delegate: ColumnLayout {
                        id: group
                        required property var modelData
                        Layout.fillWidth: true
                        spacing: 6

                        DText {
                            text: group.modelData.title.toUpperCase()
                            faint: true
                            font.pixelSize: Theme.textXs
                            font.weight: Font.DemiBold
                            font.letterSpacing: 1
                        }
                        Flow {
                            Layout.fillWidth: true
                            spacing: 8
                            Repeater {
                                model: group.modelData.ids
                                delegate: DButton {
                                    required property string modelData
                                    text: Learn.get(modelData) ? Learn.get(modelData).title : modelData
                                    onClicked: browser.openLesson(modelData)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

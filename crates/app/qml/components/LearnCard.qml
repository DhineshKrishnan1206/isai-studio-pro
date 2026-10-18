import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// A plain-English lesson about one topic (see Learn.qml), over a dimmed backdrop.
// Esc or a click outside closes it.
Popup {
    id: card
    /// The topic id in Learn.qml, e.g. "eq" or "mixer".
    property string topic: ""
    property Item backdrop: null
    readonly property var lesson: Learn.get(card.topic)

    /// The defined words that appear in this lesson, in Learn.terms order.
    readonly property var usedTerms: {
        if (!card.lesson) return []
        const text = [card.lesson.what, card.lesson.tip, card.lesson.example].concat(card.lesson.how).join(" ")
        const found = []
        for (const word of Object.keys(Learn.terms)) {
            if (new RegExp("(^|[^A-Za-z])" + word + "s?([^A-Za-z]|$)", "i").test(text))
                found.push({ word: word, meaning: Learn.terms[word] })
        }
        return found
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(480, (parent ? parent.width : 600) - 48)
    padding: 22
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    Overlay.modal: Rectangle { color: Qt.rgba(0, 0, 0, 0.35) }
    background: GlassPanel { backdrop: card.backdrop; radius: Theme.radiusXl; clarity: 0.25 }
    enter: Transition { NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.fast } }
    exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: Theme.fast } }

    contentItem: ColumnLayout {
        spacing: 12

        RowLayout {
            Layout.fillWidth: true
            spacing: 10
            Icon { glyph: Icons.info; size: 18; color: Theme.accent }
            DText {
                Layout.fillWidth: true
                text: card.lesson ? card.lesson.title : "Lesson not found"
                strong: true
                font.pixelSize: Theme.textLg
            }
            IconButton { glyph: Icons.x; tip: "Close (Esc)"; onClicked: card.close() }
        }

        DText {
            Layout.fillWidth: true
            visible: !!card.lesson
            text: card.lesson ? card.lesson.what : ""
            wrapMode: Text.WordWrap
            font.pixelSize: Theme.textMd
        }

        DText {
            visible: !!card.lesson
            text: "HOW IT WORKS"
            faint: true
            font.pixelSize: Theme.textXs
            font.weight: Font.DemiBold
            font.letterSpacing: 1
        }

        Repeater {
            model: card.lesson ? card.lesson.how : []
            delegate: RowLayout {
                id: step
                required property string modelData
                required property int index
                Layout.fillWidth: true
                spacing: 10
                DText {
                    text: (step.index + 1) + "."
                    font.family: Theme.mono
                    dim: true
                    Layout.alignment: Qt.AlignTop
                }
                DText {
                    Layout.fillWidth: true
                    text: step.modelData
                    wrapMode: Text.WordWrap
                    dim: true
                }
            }
        }

        Rectangle {
            visible: !!card.lesson
            Layout.fillWidth: true
            implicitHeight: tipText.implicitHeight + 20
            radius: Theme.radiusMd
            color: Theme.tint(Theme.accent, 0.12)
            border.color: Theme.tint(Theme.accent, 0.4)
            DText {
                id: tipText
                anchors.fill: parent
                anchors.margins: 10
                text: card.lesson ? "Tip: " + card.lesson.tip : ""
                wrapMode: Text.WordWrap
                font.pixelSize: Theme.textSm
            }
        }

        ColumnLayout {
            visible: card.usedTerms.length > 0
            Layout.fillWidth: true
            spacing: 6
            DText {
                text: "WORDS IN THIS LESSON"
                faint: true
                font.pixelSize: Theme.textXs
                font.weight: Font.DemiBold
                font.letterSpacing: 1
            }
            Repeater {
                model: card.usedTerms
                delegate: DText {
                    required property var modelData
                    Layout.fillWidth: true
                    text: "<b>" + modelData.word + "</b>: " + modelData.meaning
                    textFormat: Text.RichText
                    wrapMode: Text.WordWrap
                    dim: true
                    font.pixelSize: Theme.textSm
                }
            }
        }

        DText {
            visible: !!card.lesson && !!card.lesson.example
            Layout.fillWidth: true
            text: card.lesson ? "Example: " + card.lesson.example : ""
            wrapMode: Text.WordWrap
            faint: true
            font.pixelSize: Theme.textSm
        }
    }
}

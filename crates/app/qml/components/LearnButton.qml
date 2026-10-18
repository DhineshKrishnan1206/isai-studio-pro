import QtQuick
import Isai

// A "?" button: click it to read a plain-English lesson about `topic`.
IconButton {
    id: button
    /// The topic id in Learn.qml.
    property string topic: ""
    /// What the lesson's glass card blurs (usually the page behind it).
    property Item backdrop: null

    glyph: Icons.info
    tip: "What is this? Click to learn"
    onClicked: card.open()

    LearnCard { id: card; topic: button.topic; backdrop: button.backdrop }
}

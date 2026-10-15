import QtQuick
import Isai

// Text with the app's font and colours. Use this instead of plain Text.
Text {
    property bool strong: false
    property bool dim: false
    property bool faint: false

    font.family: Theme.font
    font.pixelSize: Theme.textMd
    font.weight: strong ? Font.DemiBold : Font.Normal
    color: faint ? Theme.textFaint : dim ? Theme.textDim : Theme.text
    verticalAlignment: Text.AlignVCenter
    textFormat: Text.PlainText
    Behavior on color { ColorAnimation { duration: Theme.fast } }
}

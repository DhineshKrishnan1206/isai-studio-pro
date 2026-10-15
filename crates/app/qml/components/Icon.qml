import QtQuick
import Isai

// A Phosphor icon. Set `glyph` to a value from Icons, e.g. `Icons.play`.
Text {
    property string glyph
    property bool filled: false
    property real size: 18

    text: glyph
    font.family: filled ? Theme.iconFillFont : Theme.iconFont
    font.pixelSize: size
    color: Theme.text
    horizontalAlignment: Text.AlignHCenter
    verticalAlignment: Text.AlignVCenter
    Behavior on color { ColorAnimation { duration: Theme.fast } }
}

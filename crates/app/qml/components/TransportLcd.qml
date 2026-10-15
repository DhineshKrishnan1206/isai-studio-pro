import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// The control bar's LCD: position, tempo, time signature and key.
//   • Click the position to switch between bars/beats and minutes/seconds.
//   • Drag the tempo up or down (Shift for fine steps), scroll it, or
//     double-click to type a value.
//   • Click the time signature or key to choose from a list.
Rectangle {
    id: lcd
    property var app
    property bool showTime: false

    implicitWidth: row.implicitWidth + 20
    implicitHeight: 42
    radius: Theme.radiusMd
    color: Theme.lcd
    border.color: Theme.studio ? "#000000" : Theme.line

    readonly property var bars: {
        const parts = app.barText.trim().split(/\s+/)
        return parts.length === 4 ? parts : ["1", "1", "1", "0"]
    }

    // A value with a tiny caption under it, like a hardware display.
    component Field: Item {
        id: field
        property alias value: valueText.text
        property alias caption: captionText.text
        property int size: 19
        property color tint: Theme.lcdText
        implicitWidth: Math.max(valueText.implicitWidth, captionText.implicitWidth)
        implicitHeight: 36
        DText {
            id: valueText
            anchors.horizontalCenter: parent.horizontalCenter
            y: 0
            height: 24
            font.family: Theme.mono
            font.pixelSize: field.size
            color: field.tint
            verticalAlignment: Text.AlignBottom
        }
        DText {
            id: captionText
            anchors.horizontalCenter: parent.horizontalCenter
            y: 25
            font.pixelSize: 8
            font.weight: Font.DemiBold
            font.letterSpacing: 0.8
            color: Theme.lcdDim
        }
    }
    component Divider: Rectangle {
        Layout.preferredWidth: 1
        Layout.preferredHeight: 26
        Layout.leftMargin: 12
        Layout.rightMargin: 12
        color: Theme.tint(Theme.lcdDim, 0.35)
    }

    RowLayout {
        id: row
        anchors.centerIn: parent
        spacing: 0

        // Position: bars/beats or time
        Item {
            implicitWidth: lcd.showTime ? timeField.width : barRow.width
            implicitHeight: 36
            Row {
                id: barRow
                visible: !lcd.showTime
                anchors.verticalCenter: parent.verticalCenter
                spacing: 10
                Field { value: lcd.bars[0]; caption: lcd.app.countingIn ? "COUNT" : "BAR"; tint: lcd.app.countingIn ? Theme.loopOn : Theme.lcdText }
                Field { value: lcd.bars[1]; caption: "BEAT" }
                Field { value: lcd.bars[2]; caption: "DIV" }
                Field { value: ("00" + lcd.bars[3]).slice(-3); caption: "TICK"; size: 14 }
            }
            Field {
                id: timeField
                visible: lcd.showTime
                anchors.verticalCenter: parent.verticalCenter
                value: lcd.app.positionText
                caption: "TIME"
            }
            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: lcd.showTime = !lcd.showTime
            }
        }

        Divider {}

        // Tempo
        Item {
            id: tempoBox
            property bool editing: false
            implicitWidth: 64
            implicitHeight: 36
            Field {
                visible: !tempoBox.editing
                anchors.centerIn: parent
                value: lcd.app.tempo.toFixed(1)
                caption: "TEMPO"
            }
            TextInput {
                id: tempoInput
                visible: tempoBox.editing
                anchors.centerIn: parent
                width: 64
                font.family: Theme.mono
                font.pixelSize: 19
                color: Theme.lcdText
                selectionColor: Theme.accent
                horizontalAlignment: TextInput.AlignHCenter
                validator: DoubleValidator { bottom: 20; top: 400; decimals: 1 }
                onAccepted: { lcd.app.setProjectTempo(parseFloat(text)); tempoBox.editing = false }
                onActiveFocusChanged: if (!activeFocus) tempoBox.editing = false
                Keys.onEscapePressed: tempoBox.editing = false
            }
            MouseArea {
                anchors.fill: parent
                enabled: !tempoBox.editing
                hoverEnabled: true
                cursorShape: Qt.SizeVerCursor
                property real startY: 0
                property real startTempo: 0
                onPressed: m => { startY = m.y; startTempo = lcd.app.tempo }
                onPositionChanged: m => {
                    if (!pressed) return
                    const step = (m.modifiers & Qt.ShiftModifier) ? 0.1 : 1
                    lcd.app.setProjectTempo(startTempo + Math.round((startY - m.y) / 3) * step)
                }
                onDoubleClicked: {
                    tempoBox.editing = true
                    tempoInput.text = lcd.app.tempo.toFixed(1)
                    tempoInput.forceActiveFocus()
                    tempoInput.selectAll()
                }
                onWheel: w => lcd.app.setProjectTempo(lcd.app.tempo + (w.angleDelta.y > 0 ? 1 : -1))
                DToolTip {
                    visible: parent.containsMouse && !parent.pressed
                    text: "Drag or scroll to change · Shift: fine · Double-click: type · T: tap tempo"
                }
            }
        }

        Divider {}

        // Time signature
        Field {
            value: lcd.app.timeSigNum + "/" + lcd.app.timeSigDen
            caption: "SIG"
            size: 16
            MouseArea {
                anchors.fill: parent
                anchors.margins: -6
                cursorShape: Qt.PointingHandCursor
                onClicked: sigPopup.open()
            }
        }

        Item { implicitWidth: 16 }

        // Key
        Field {
            value: lcd.app.keyName.replace(" major", "").replace(" minor", "m")
            caption: "KEY"
            size: 16
            MouseArea {
                anchors.fill: parent
                anchors.margins: -6
                cursorShape: Qt.PointingHandCursor
                onClicked: keyPopup.open()
            }
        }
    }

    ChoicePopup {
        id: sigPopup
        y: lcd.height + 6
        x: lcd.width - width
        title: "TIME SIGNATURE"
        columns: 4
        options: ["2/4", "3/4", "4/4", "5/4", "6/8", "7/8", "9/8", "12/8"].map(s => ({ value: s, label: s }))
        current: lcd.app.timeSigNum + "/" + lcd.app.timeSigDen
        onPicked: v => {
            const parts = v.split("/")
            lcd.app.setTimeSignature(parseInt(parts[0]), parseInt(parts[1]))
        }
    }

    ChoicePopup {
        id: keyPopup
        y: lcd.height + 6
        x: lcd.width - width
        title: "KEY"
        columns: 6
        readonly property var notes: ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"]
        options: notes.map(n => ({ value: n + " major", label: n }))
                      .concat(notes.map(n => ({ value: n + " minor", label: n + "m" })))
        current: lcd.app.keyName
        onPicked: v => lcd.app.setProjectKey(v)
    }
}

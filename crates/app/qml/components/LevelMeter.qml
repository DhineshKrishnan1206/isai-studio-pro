import QtQuick
import Isai

// Stereo level meter. `leftLevel`/`rightLevel` are 0..1 on a -60..0 dB scale.
// The coloured gradient is drawn once; only the clip widths change.
Column {
    id: meter
    property real leftLevel: 0
    property real rightLevel: 0
    property real meterWidth: 140
    spacing: 3

    Repeater {
        model: [meter.leftLevel, meter.rightLevel]
        delegate: Rectangle {
            required property real modelData
            width: meter.meterWidth
            height: 5
            radius: 2.5
            color: Theme.sunken
            border.color: Theme.line

            Item {
                width: parent.width * Math.max(0, Math.min(1, parent.modelData))
                height: parent.height
                clip: true
                Rectangle {
                    width: meter.meterWidth
                    height: parent.height
                    radius: 2.5
                    gradient: Gradient {
                        orientation: Gradient.Horizontal
                        GradientStop { position: 0.0; color: Theme.success }
                        GradientStop { position: 0.72; color: Theme.success }
                        GradientStop { position: 0.86; color: Theme.warning }
                        GradientStop { position: 1.0; color: Theme.danger }
                    }
                }
            }
        }
    }
}

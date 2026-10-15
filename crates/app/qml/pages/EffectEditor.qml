import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Isai

// A full editor for one effect, in a floating glass window (drag its title
// bar to move it). Input and output meters on the right; the body depends
// on the effect:
//   • EQ: a graph with a smooth analyzer and the curve, plus a strip of band
//     columns. Drag a numbered handle to set frequency and gain, drag the
//     selected band's side handles (or scroll) for width or slope, double-click
//     a handle to switch the band on or off, double-click empty space to add a
//     band. Each column has a gain fader and frequency and width knobs.
//   • Compressor: a hardware-style unit with a needle gain-reduction meter,
//     metal knobs, the transfer curve, and Peak/RMS, Auto gain and bypass keys.
//   • Limiter: a gain-reduction meter.
Item {
    id: editor
    property var app
    /// The effect (an entry of some chain's "inserts"), or null when closed.
    property var insert: null
    property string chainName: ""
    property color tint: Theme.accent
    property Item backdrop: null
    signal closeRequested()

    readonly property bool shown: !!insert
    readonly property string kind: insert ? insert.kind : ""
    readonly property var def: {
        if (!insert) return null
        try { return JSON.parse(app.effectKindsJson).find(k => k.key === insert.kind) } catch (e) { return null }
    }
    function param(key) { return def ? def.params.find(p => p.key === key) : null }
    /// A parameter's current value (its default until the effect's values arrive).
    function v(key) {
        const x = values[key]
        if (x !== undefined) return x
        const d = param(key)
        return d ? d.default : 0
    }

    /// Values being dragged right now (published when the drag ends).
    property var live: ({})
    readonly property var values: insert ? Object.assign({}, insert.values, live) : ({})
    onInsertChanged: if (!insert || (lastId !== insert.id)) { live = {}; lastId = insert ? insert.id : -1 }
    property int lastId: -1
    function set(key, v, done) {
        const l = Object.assign({}, live)
        if (done) delete l[key]
        else l[key] = v
        live = l
        app.setInsertParam(insert.id, key, v, done)
    }

    width: kind === "eq" ? 1000 : kind === "compressor" ? 860 : 680
    height: kind === "eq" ? 660 : kind === "compressor" ? 470 : 400
    x: (parent.width - width) / 2
    y: Math.max(12, (parent.height - height) / 2 - 40)
    visible: opacity > 0
    opacity: shown ? 1 : 0
    scale: shown ? 1 : 0.97
    Behavior on opacity { NumberAnimation { duration: Theme.fast } }
    Behavior on scale { NumberAnimation { duration: Theme.slow; easing.type: Easing.OutCubic } }
    z: 150

    // Live readings, polled while open.
    property var meters: ({ in: [0, 0], out: [0, 0], gr: 0 })
    Timer {
        running: editor.shown && editor.visible
        repeat: true
        interval: 33
        onTriggered: {
            try { editor.meters = JSON.parse(editor.app.effectMeters(editor.insert.id) || "{}") } catch (e) {}
            if (editor.kind === "eq" && editor.analyzer > 0) {
                try {
                    const s = JSON.parse(editor.app.effectSpectra(editor.insert.id, 128) || "{}")
                    if (s.post) {
                        if (editor.analyzer === 2) editor.specPre = editor.smooth(editor.specPre, s.pre)
                        editor.specPost = editor.smooth(editor.specPost, s.post)
                    }
                } catch (e) {}
            }
        }
    }
    // Peak hold for gain reduction, so short dips are readable.
    property real grHold: 0
    onMetersChanged: grHold = Math.min(0, Math.max((meters.gr || 0), grHold * 0.9 + (meters.gr || 0) * 0.1))

    GlassPanel {
        anchors.fill: parent
        backdrop: editor.backdrop
        clarity: 0.22
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons; onWheel: w => w.accepted = true }

        // ------------------------------------------------ title bar
        Item {
            id: titleBar
            width: parent.width
            height: 46
            DragHandler {
                target: editor
                // Free to move, as long as part of the title bar stays on screen.
                xAxis.minimum: 120 - editor.width; xAxis.maximum: editor.parent.width - 120
                yAxis.minimum: 0; yAxis.maximum: editor.parent.height - 50
            }
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 10
                spacing: 10
                // On / bypass
                AbstractButton {
                    implicitWidth: 22; implicitHeight: 22
                    hoverEnabled: true
                    onClicked: editor.app.toggleInsert(editor.insert.id)
                    background: Rectangle {
                        radius: 11
                        color: "transparent"
                        border.color: editor.insert && editor.insert.enabled ? editor.tint : Theme.lineStrong
                        border.width: 2
                        Rectangle {
                            anchors.centerIn: parent
                            width: 10; height: 10; radius: 5
                            color: editor.insert && editor.insert.enabled ? editor.tint : "transparent"
                        }
                    }
                    DToolTip { visible: parent.hovered; text: editor.insert && editor.insert.enabled ? "On · click to bypass" : "Bypassed · click to turn on" }
                }
                DText { text: editor.insert ? editor.insert.label : ""; strong: true; font.pixelSize: Theme.textLg }
                DText { text: editor.chainName; faint: true; font.pixelSize: Theme.textSm }
                Item { Layout.fillWidth: true }
                IconButton {
                    implicitWidth: 28; implicitHeight: 28; iconSize: 15
                    glyph: Icons.x
                    tip: "Close"
                    onClicked: editor.closeRequested()
                }
            }
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.line }
        }

        RowLayout {
            anchors.fill: parent
            anchors.topMargin: titleBar.height + 10
            anchors.margins: 14
            spacing: 14

            // ------------------------------------------------ body
            Loader {
                Layout.fillWidth: true
                Layout.fillHeight: true
                sourceComponent: editor.kind === "eq" ? eqBody
                               : editor.kind === "compressor" ? compBody
                               : editor.kind === "limiter" ? limiterBody
                               : knobsBody
            }

            // ------------------------------------------------ meters
            Rectangle { Layout.preferredWidth: 1; Layout.fillHeight: true; color: Theme.line }
            ColumnLayout {
                Layout.fillHeight: true
                spacing: 6
                RowLayout {
                    spacing: 12
                    Layout.fillHeight: true
                    StereoMeter { label: "IN"; levels: editor.meters.in || [0, 0]; Layout.fillHeight: true }
                    StereoMeter { label: "OUT"; levels: editor.meters.out || [0, 0]; Layout.fillHeight: true }
                }
            }
        }
    }

    // ================================================================ EQ
    // In the spirit of FL Studio's Parametric EQ 2: a dark graph with a smooth
    // analyzer, the curve in white, numbered handles in each band's colour,
    // and a strip of band columns underneath (gain fader, frequency, width).
    /// Analyzer: 0 off, 1 after the EQ only, 2 before (line) and after (filled).
    property int analyzer: 1
    // Smoothed spectra (quick rise, slow fall), so the analyzer flows.
    property var specPre: []
    property var specPost: []
    /// A CSS colour string for Canvas from a colour and an opacity.
    function rgba(c, a) {
        const q = Qt.color(c)
        return "rgba(" + Math.round(q.r * 255) + "," + Math.round(q.g * 255) + "," + Math.round(q.b * 255) + "," + a + ")"
    }
    function smooth(old, fresh) {
        const out = new Array(fresh.length)
        for (let i = 0; i < fresh.length; ++i) {
            const s = old.length === fresh.length ? old[i] : -100
            const v = fresh[i]
            out[i] = v > s ? s + (v - s) * 0.45 : s + (v - s) * 0.12
        }
        return out
    }
    /// Draws a smooth curve through `pts` ([x, y] pairs), Catmull-Rom style.
    /// `join` continues the current outline instead of starting a new one.
    function spline(ctx, pts, join) {
        const n = pts.length
        if (n < 2) return
        if (join) ctx.lineTo(pts[0][0], pts[0][1]); else ctx.moveTo(pts[0][0], pts[0][1])
        for (let i = 0; i < n - 1; ++i) {
            const p0 = pts[Math.max(0, i - 1)], p1 = pts[i], p2 = pts[i + 1], p3 = pts[Math.min(n - 1, i + 2)]
            ctx.bezierCurveTo(p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6,
                              p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6, p2[0], p2[1])
        }
    }
    // ---------------------------------------------------------- EQ presets
    // Starting points, grouped like a preset browser. Each starts from a flat
    // EQ and sets only what it needs. Slopes: 0 = 6, 1 = 12, 2 = 24, 3 = 48 dB/oct.
    readonly property var eqPresets: [
        { group: "Vocals", items: [
            { name: "Vocal clarity", v: { lc_on: 1, lc_freq: 90, lc_slope: 2, b1_on: 1, b1_freq: 250, b1_gain: -2.5, b1_q: 1.2, b2_on: 1, b2_freq: 3000, b2_gain: 2.5, b2_q: 1, hs_on: 1, hs_freq: 10000, hs_gain: 3 } },
            { name: "Warm vocal", v: { lc_on: 1, lc_freq: 75, lc_slope: 2, ls_on: 1, ls_freq: 180, ls_gain: 2, b2_on: 1, b2_freq: 2800, b2_gain: -1.5, b2_q: 1, hc_on: 1, hc_freq: 14000, hc_slope: 1 } },
            { name: "Human voice", v: { lc_on: 1, lc_freq: 80, lc_slope: 2, b1_on: 1, b1_freq: 200, b1_gain: 1.5, b1_q: 0.9, b2_on: 1, b2_freq: 500, b2_gain: -2, b2_q: 1.4, b3_on: 1, b3_freq: 4000, b3_gain: 2, b3_q: 1 } },
            { name: "Podcast / speech", v: { lc_on: 1, lc_freq: 100, lc_slope: 2, b1_on: 1, b1_freq: 300, b1_gain: -3, b1_q: 1.4, b2_on: 1, b2_freq: 2500, b2_gain: 3, b2_q: 1, b3_on: 1, b3_freq: 6500, b3_gain: -2, b3_q: 3, hc_on: 1, hc_freq: 15000, hc_slope: 1 } },
            { name: "De-ess (soften S)", v: { b3_on: 1, b3_freq: 6800, b3_gain: -4.5, b3_q: 3.5 } },
            { name: "Backing vocals", v: { lc_on: 1, lc_freq: 150, lc_slope: 2, b2_on: 1, b2_freq: 2500, b2_gain: -2, b2_q: 0.8, hs_on: 1, hs_freq: 11000, hs_gain: 2.5 } }
        ] },
        { group: "Effects", items: [
            { name: "Radio", v: { lc_on: 1, lc_freq: 400, lc_slope: 2, hc_on: 1, hc_freq: 4000, hc_slope: 2, b2_on: 1, b2_freq: 1500, b2_gain: 5, b2_q: 1.1, output: 2 } },
            { name: "Telephone", v: { lc_on: 1, lc_freq: 500, lc_slope: 3, hc_on: 1, hc_freq: 3000, hc_slope: 3, b2_on: 1, b2_freq: 1200, b2_gain: 6, b2_q: 1.4, output: 3 } },
            { name: "Old AM radio", v: { lc_on: 1, lc_freq: 250, lc_slope: 1, hc_on: 1, hc_freq: 5000, hc_slope: 1, b2_on: 1, b2_freq: 900, b2_gain: 3, b2_q: 0.8 } },
            { name: "Megaphone", v: { lc_on: 1, lc_freq: 700, lc_slope: 3, hc_on: 1, hc_freq: 3500, hc_slope: 3, b2_on: 1, b2_freq: 1800, b2_gain: 9, b2_q: 2, output: -3 } },
            { name: "Next room", v: { hc_on: 1, hc_freq: 900, hc_slope: 2, ls_on: 1, ls_freq: 150, ls_gain: 2, output: -2 } },
            { name: "Lo-fi", v: { lc_on: 1, lc_freq: 120, lc_slope: 1, hc_on: 1, hc_freq: 6500, hc_slope: 1, b2_on: 1, b2_freq: 1200, b2_gain: 2, b2_q: 0.7 } }
        ] },
        { group: "Drums & bass", items: [
            { name: "Kick punch", v: { lc_on: 1, lc_freq: 30, lc_slope: 2, b1_on: 1, b1_freq: 60, b1_gain: 4, b1_q: 1.2, b2_on: 1, b2_freq: 350, b2_gain: -4, b2_q: 1.5, b3_on: 1, b3_freq: 4000, b3_gain: 3, b3_q: 1.4 } },
            { name: "Snare snap", v: { lc_on: 1, lc_freq: 80, lc_slope: 2, b1_on: 1, b1_freq: 200, b1_gain: 2, b1_q: 1, b2_on: 1, b2_freq: 900, b2_gain: -2, b2_q: 1.2, b3_on: 1, b3_freq: 5000, b3_gain: 3, b3_q: 1, hs_on: 1, hs_freq: 10000, hs_gain: 2 } },
            { name: "Hi-hats & cymbals", v: { lc_on: 1, lc_freq: 300, lc_slope: 2, b3_on: 1, b3_freq: 3500, b3_gain: -2, b3_q: 1.4, hs_on: 1, hs_freq: 12000, hs_gain: 2.5 } },
            { name: "Drum bus", v: { lc_on: 1, lc_freq: 30, lc_slope: 1, ls_on: 1, ls_freq: 80, ls_gain: 2, b2_on: 1, b2_freq: 400, b2_gain: -2, b2_q: 1, hs_on: 1, hs_freq: 9000, hs_gain: 2 } },
            { name: "Tight bass", v: { lc_on: 1, lc_freq: 35, lc_slope: 2, b1_on: 1, b1_freq: 80, b1_gain: 2, b1_q: 1, b2_on: 1, b2_freq: 250, b2_gain: -3, b2_q: 1.2, b3_on: 1, b3_freq: 800, b3_gain: 2, b3_q: 1.5, hc_on: 1, hc_freq: 8000, hc_slope: 1 } },
            { name: "808 / sub", v: { lc_on: 1, lc_freq: 28, lc_slope: 2, b1_on: 1, b1_freq: 55, b1_gain: 3, b1_q: 1, b2_on: 1, b2_freq: 300, b2_gain: -3, b2_q: 1, hc_on: 1, hc_freq: 5000, hc_slope: 2 } }
        ] },
        { group: "Instruments", items: [
            { name: "Acoustic guitar", v: { lc_on: 1, lc_freq: 80, lc_slope: 2, b1_on: 1, b1_freq: 200, b1_gain: -2.5, b1_q: 1, b3_on: 1, b3_freq: 5000, b3_gain: 2, b3_q: 0.9, hs_on: 1, hs_freq: 12000, hs_gain: 2 } },
            { name: "Electric guitar", v: { lc_on: 1, lc_freq: 100, lc_slope: 2, b1_on: 1, b1_freq: 400, b1_gain: -2, b1_q: 1, b2_on: 1, b2_freq: 2500, b2_gain: 2, b2_q: 1.2, hc_on: 1, hc_freq: 9000, hc_slope: 1 } },
            { name: "Piano", v: { lc_on: 1, lc_freq: 40, lc_slope: 1, b1_on: 1, b1_freq: 300, b1_gain: -1.5, b1_q: 1, b3_on: 1, b3_freq: 3000, b3_gain: 1.5, b3_q: 0.9, hs_on: 1, hs_freq: 9000, hs_gain: 2 } },
            { name: "Synth pad", v: { lc_on: 1, lc_freq: 150, lc_slope: 2, b2_on: 1, b2_freq: 600, b2_gain: -2, b2_q: 0.8, hs_on: 1, hs_freq: 8000, hs_gain: 2.5 } },
            { name: "Strings", v: { lc_on: 1, lc_freq: 60, lc_slope: 1, b1_on: 1, b1_freq: 250, b1_gain: -1.5, b1_q: 1, b3_on: 1, b3_freq: 2500, b3_gain: -1.5, b3_q: 1.2, hs_on: 1, hs_freq: 10000, hs_gain: 2 } }
        ] },
        { group: "Mix & master", items: [
            { name: "Smile (loudness)", v: { ls_on: 1, ls_freq: 100, ls_gain: 3, b2_on: 1, b2_freq: 1000, b2_gain: -2, b2_q: 0.7, hs_on: 1, hs_freq: 8000, hs_gain: 3 } },
            { name: "Gentle master", v: { lc_on: 1, lc_freq: 25, lc_slope: 1, ls_on: 1, ls_freq: 60, ls_gain: 1, b2_on: 1, b2_freq: 3000, b2_gain: -1, b2_q: 0.8, hs_on: 1, hs_freq: 12000, hs_gain: 1.5 } },
            { name: "Add air", v: { hs_on: 1, hs_freq: 12000, hs_gain: 4 } },
            { name: "Add warmth", v: { ls_on: 1, ls_freq: 200, ls_gain: 2.5, hs_on: 1, hs_freq: 9000, hs_gain: -1.5 } },
            { name: "Cut mud", v: { b1_on: 1, b1_freq: 300, b1_gain: -4, b1_q: 1.2 } },
            { name: "Tame harshness", v: { b3_on: 1, b3_freq: 3200, b3_gain: -3, b3_q: 1.4 } }
        ] },
        { group: "Filters", items: [
            { name: "Flat (reset)", v: {} },
            { name: "Remove rumble", v: { lc_on: 1, lc_freq: 40, lc_slope: 2 } },
            { name: "Low cut 100 Hz", v: { lc_on: 1, lc_freq: 100, lc_slope: 2 } },
            { name: "High cut 10 kHz", v: { hc_on: 1, hc_freq: 10000, hc_slope: 1 } },
            { name: "Band pass (mids)", v: { lc_on: 1, lc_freq: 300, lc_slope: 2, hc_on: 1, hc_freq: 3000, hc_slope: 2 } },
            { name: "Remove hum 50 Hz", v: { b1_on: 1, b1_freq: 50, b1_gain: -18, b1_q: 8, b2_on: 1, b2_freq: 100, b2_gain: -12, b2_q: 8 } },
            { name: "Remove hum 60 Hz", v: { b1_on: 1, b1_freq: 60, b1_gain: -18, b1_q: 8, b2_on: 1, b2_freq: 120, b2_gain: -12, b2_q: 8 } }
        ] }
    ]
    /// The last preset loaded into this editor, and whether it was changed since.
    property string presetName: ""
    property var presetValues: null
    readonly property bool presetEdited: {
        if (!presetValues) return false
        for (const k in presetValues) if (Math.abs((values[k] === undefined ? presetValues[k] : values[k]) - presetValues[k]) > 1e-3) return true
        return false
    }
    onLastIdChanged: { presetName = ""; presetValues = null }
    /// Loads a preset over a flat EQ, as one undo step.
    function loadPreset(p) {
        const all = {}
        for (const d of def.params) all[d.key] = d.default
        Object.assign(all, p.v)
        live = {}
        app.setInsertParams(insert.id, JSON.stringify(all), "EQ Preset: " + p.name)
        presetName = p.name
        presetValues = all
    }

    readonly property var eqBands: [
        { name: "Low cut", type: "Low cut", on: "lc_on", freq: "lc_freq", slope: "lc_slope", color: "#FF5A6E" },
        { name: "Low shelf", type: "Low shelf", on: "ls_on", freq: "ls_freq", gain: "ls_gain", q: "ls_q", color: "#FF9A3D" },
        { name: "Band 3", type: "Peaking", on: "b1_on", freq: "b1_freq", gain: "b1_gain", q: "b1_q", color: "#FFD43B" },
        { name: "Band 4", type: "Peaking", on: "b2_on", freq: "b2_freq", gain: "b2_gain", q: "b2_q", color: "#7ED957" },
        { name: "Band 5", type: "Peaking", on: "b3_on", freq: "b3_freq", gain: "b3_gain", q: "b3_q", color: "#34D1C9" },
        { name: "Band 6", type: "Peaking", on: "b4_on", freq: "b4_freq", gain: "b4_gain", q: "b4_q", color: "#4C9DFF" },
        { name: "High shelf", type: "High shelf", on: "hs_on", freq: "hs_freq", gain: "hs_gain", q: "hs_q", color: "#9D7BFF" },
        { name: "High cut", type: "High cut", on: "hc_on", freq: "hc_freq", slope: "hc_slope", color: "#F062C8" }
    ]

    Component {
        id: eqBody
        Item {
            id: eqRoot
            property int selected: 2
            ColumnLayout {
                anchors.fill: parent
                spacing: 10
                EqGraph {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    bands: editor.eqBands
                    selected: eqRoot.selected
                    onPicked: i => eqRoot.selected = i
                }
                // One column per band.
                RowLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: false
                    Layout.preferredHeight: 176
                    spacing: 6
                    Repeater {
                        model: editor.eqBands
                        delegate: BandColumn {
                            required property var modelData
                            required property int index
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            band: modelData
                            number: index + 1
                            current: eqRoot.selected === index
                            onPicked: eqRoot.selected = index
                        }
                    }
                }
            }
        }
    }

    // A band's column in the strip: its number and type (click to pick,
    // the dot switches it on/off), a gain fader, frequency and width knobs.
    component BandColumn: Rectangle {
        id: col
        property var band
        property int number
        property bool current: false
        signal picked()
        readonly property bool on: editor.v(band.on) >= 0.5
        radius: 8
        color: current ? Theme.tint(band.color, 0.1) : "#1A1D22"
        border.color: current ? Theme.tint(band.color, 0.7) : "#262A31"
        Behavior on color { ColorAnimation { duration: Theme.fast } }
        MouseArea { anchors.fill: parent; onPressed: col.picked() }

        RowLayout {
            id: head
            x: 8; y: 6
            width: parent.width - 16
            height: 20
            spacing: 6
            Rectangle {
                implicitWidth: 18; implicitHeight: 18; radius: 9
                color: col.on ? col.band.color : "transparent"
                border.color: col.band.color
                border.width: 1.5
                DText {
                    anchors.centerIn: parent
                    text: col.number
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    color: col.on ? "#111317" : col.band.color
                }
                MouseArea {
                    anchors.fill: parent
                    anchors.margins: -3
                    cursorShape: Qt.PointingHandCursor
                    onClicked: { col.picked(); editor.set(col.band.on, col.on ? 0 : 1, true) }
                }
            }
            DText {
                Layout.fillWidth: true
                text: col.band.type
                elide: Text.ElideRight
                font.pixelSize: 10
                font.weight: Font.DemiBold
                color: col.on ? "#E6E8EC" : "#6B7079"
            }
        }
        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            y: head.y + head.height + 8
            spacing: 4
            opacity: col.on ? 1 : 0.45
            Behavior on opacity { NumberAnimation { duration: Theme.fast } }
            GainFader {
                visible: !!col.band.gain
                key: col.band.gain || ""
                onKey: col.band.on
                tint: col.band.color
                height: 128
            }
            Column {
                spacing: 0
                ParamKnob {
                    def: editor.param(col.band.freq)
                    value: editor.v(col.band.freq)
                    tint: col.band.color
                    label: "Freq"
                    onMoved: (v, done) => { col.picked(); editor.set(col.band.on, 1, done); editor.set(col.band.freq, v, done) }
                }
                ParamKnob {
                    readonly property string key: col.band.q || col.band.slope
                    def: editor.param(key)
                    value: editor.v(key)
                    tint: col.band.color
                    label: col.band.q ? "Width" : "Slope"
                    onMoved: (v, done) => { col.picked(); editor.set(key, v, done) }
                }
            }
        }
    }

    // A vertical gain fader centred on 0 dB. Drag (⇧ fine), scroll, double-click for 0.
    component GainFader: Item {
        id: gf
        property string key
        property string onKey
        property color tint: Theme.accent
        readonly property var def: key ? editor.param(key) : null
        readonly property real value: key ? editor.v(key) : 0
        readonly property real span: def ? def.max : 24
        implicitWidth: 22
        readonly property real trackH: height - 18
        function yOf(v) { return trackH / 2 - v / span * (trackH / 2) }
        Rectangle {
            id: track
            anchors.horizontalCenter: parent.horizontalCenter
            width: 4; height: gf.trackH
            radius: 2
            color: "#0E1013"
            Rectangle { y: gf.trackH / 2; width: parent.width; height: 1; color: "#3A3F47" }
            Rectangle {
                width: parent.width
                y: Math.min(gf.yOf(gf.value), gf.trackH / 2)
                height: Math.abs(gf.yOf(gf.value) - gf.trackH / 2)
                radius: 2
                color: gf.tint
            }
        }
        Rectangle {
            anchors.horizontalCenter: parent.horizontalCenter
            y: gf.yOf(gf.value) - height / 2
            width: 18; height: 9; radius: 3
            color: area.pressed ? "#FFFFFF" : "#D9DCE2"
            border.color: "#0E1013"
        }
        DText {
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            text: (gf.value > 0.05 ? "+" : "") + gf.value.toFixed(1)
            font.family: Theme.mono
            font.pixelSize: 9
            color: "#AEB3BC"
        }
        MouseArea {
            id: area
            width: parent.width + 8; x: -4
            height: gf.trackH
            preventStealing: true
            cursorShape: Qt.SizeVerCursor
            property real startY
            property real startValue
            onPressed: m => { startY = m.y; startValue = gf.value }
            onPositionChanged: m => {
                if (!pressed || !gf.def) return
                const fine = m.modifiers & Qt.ShiftModifier ? 0.25 : 1
                const v = startValue + (startY - m.y) / (gf.trackH / 2) * gf.span * fine
                editor.set(gf.onKey, 1, false)
                editor.set(gf.key, Math.round(Math.max(gf.def.min, Math.min(gf.def.max, v)) * 10) / 10, false)
            }
            onReleased: { editor.set(gf.onKey, 1, true); editor.set(gf.key, gf.value, true) }
            onDoubleClicked: editor.set(gf.key, 0, true)
            onWheel: w => {
                if (!gf.def) return
                const steps = w.angleDelta.y !== 0 ? w.angleDelta.y / 120 : w.pixelDelta.y / 30
                editor.set(gf.key, Math.max(gf.def.min, Math.min(gf.def.max, Math.round((gf.value + steps * 0.5) * 10) / 10)), true)
            }
        }
    }

    // A value you drag up/down (double-click resets), for compact spots.
    component DragValue: Item {
        id: dv
        property string key
        readonly property var def: editor.param(key)
        readonly property real value: editor.v(key)
        implicitWidth: 74
        implicitHeight: 24
        Rectangle {
            anchors.fill: parent
            radius: 6
            color: dvArea.containsMouse || dvArea.pressed ? "#2A2E35" : "#1E2127"
            border.color: "#343941"
        }
        DText {
            anchors.centerIn: parent
            text: (dv.value > 0.05 ? "+" : "") + dv.value.toFixed(1) + " dB"
            font.family: Theme.mono
            font.pixelSize: Theme.textXs
            color: "#FFFFFF"
        }
        MouseArea {
            id: dvArea
            anchors.fill: parent
            hoverEnabled: true
            preventStealing: true
            cursorShape: Qt.SizeVerCursor
            property real startY
            property real startValue
            onPressed: m => { startY = m.y; startValue = dv.value }
            onPositionChanged: m => {
                if (!pressed || !dv.def) return
                const fine = m.modifiers & Qt.ShiftModifier ? 0.2 : 1
                const v = Math.max(dv.def.min, Math.min(dv.def.max, startValue + (startY - m.y) * 0.1 * fine))
                editor.set(dv.key, Math.round(v * 10) / 10, false)
            }
            onReleased: editor.set(dv.key, dv.value, true)
            onDoubleClicked: if (dv.def) editor.set(dv.key, dv.def.default, true)
            onWheel: w => {
                if (!dv.def) return
                const steps = w.angleDelta.y !== 0 ? w.angleDelta.y / 120 : w.pixelDelta.y / 30
                editor.set(dv.key, Math.max(dv.def.min, Math.min(dv.def.max, Math.round((dv.value + steps * 0.5) * 10) / 10)), true)
            }
        }
        DToolTip { visible: dvArea.containsMouse && !dvArea.pressed; text: "Output level · drag up/down (⇧ fine), scroll, double-click for 0 dB" }
    }

    component EqGraph: Item {
        id: graph
        property var bands: []
        property int selected: -1
        signal picked(int index)
        readonly property real range: 18
        // Room for the dB scale on the left and the frequency scale at the bottom.
        readonly property real plotX: 30
        readonly property real plotW: width - plotX - 30
        readonly property real plotH: height - 22
        clip: true

        function xOf(f) { return plotX + Math.log(Math.max(20, f) / 20) / Math.log(1000) * plotW }
        function freqAt(x) { return 20 * Math.pow(1000, Math.max(0, Math.min(1, (x - plotX) / plotW))) }
        function yOf(db) { return plotH / 2 - db / range * (plotH / 2 - 16) }
        function dbAt(y) { return Math.max(-range, Math.min(range, (plotH / 2 - y) / (plotH / 2 - 16) * range)) }
        function nodeY(b) { return b.gain ? yOf(editor.v(b.gain)) : yOf(0) }
        function fmtFreq(f) { return f >= 1000 ? (f / 1000).toFixed(f >= 10000 ? 1 : 2) + " kHz" : Math.round(f) + " Hz" }
        /// Bandwidth in octaves for a Q, and back.
        function octaves(q) { return 2 * Math.asinh(1 / (2 * q)) / Math.LN2 }
        function qFor(oct) { return 1 / (2 * Math.sinh(Math.LN2 / 2 * Math.max(0.02, oct))) }

        readonly property var curves: {
            if (editor.kind !== "eq") return { total: [], bands: [] }
            try { return JSON.parse(editor.app.eqCurves(JSON.stringify(editor.values), Math.max(2, Math.round(plotW / 4)))) }
            catch (e) { return { total: [], bands: [] } }
        }
        onCurvesChanged: curveCanvas.requestPaint()
        onSelectedChanged: curveCanvas.requestPaint()
        Connections { target: editor; function onSpecPostChanged() { specCanvas.requestPaint() } }

        // Background
        Rectangle {
            anchors.fill: parent
            radius: 10
            gradient: Gradient {
                GradientStop { position: 0; color: "#22262C" }
                GradientStop { position: 1; color: "#16191D" }
            }
            border.color: "#2C3138"
        }

        // Grid and scales (painted only when the size changes).
        Canvas {
            anchors.fill: parent
            onWidthChanged: requestPaint()
            onHeightChanged: requestPaint()
            onPaint: {
                const ctx = getContext("2d")
                ctx.reset()
                const x0 = graph.plotX, x1 = graph.plotX + graph.plotW, h = graph.plotH
                ctx.lineWidth = 1
                for (const f of [30, 40, 60, 70, 80, 90, 300, 400, 600, 700, 800, 900, 3000, 4000, 6000, 7000, 8000, 9000]) {
                    const x = Math.round(graph.xOf(f)) + 0.5
                    ctx.strokeStyle = "rgba(255,255,255,0.03)"
                    ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, h); ctx.stroke()
                }
                ctx.font = "600 10px sans-serif"
                ctx.textAlign = "center"
                for (const f of [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000]) {
                    const x = Math.round(graph.xOf(f)) + 0.5
                    ctx.strokeStyle = "rgba(255,255,255,0.07)"
                    ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, h); ctx.stroke()
                    ctx.fillStyle = "rgba(220,225,232,0.5)"
                    ctx.fillText(f >= 1000 ? (f / 1000) + "k" : String(f), x, h + 15)
                }
                for (const d of [-18, -12, -6, 6, 12, 18]) {
                    const y = Math.round(graph.yOf(d)) + 0.5
                    ctx.strokeStyle = "rgba(255,255,255,0.05)"
                    ctx.beginPath(); ctx.moveTo(x0, y); ctx.lineTo(x1, y); ctx.stroke()
                    ctx.fillStyle = "rgba(220,225,232,0.42)"
                    const t = (d > 0 ? "+" : "") + d
                    ctx.textAlign = "right"; ctx.fillText(t, x0 - 6, y + 3.5)
                    ctx.textAlign = "left"; ctx.fillText(t, x1 + 6, y + 3.5)
                }
                const y0 = Math.round(graph.yOf(0)) + 0.5
                ctx.strokeStyle = "rgba(255,255,255,0.16)"
                ctx.beginPath(); ctx.moveTo(x0, y0); ctx.lineTo(x1, y0); ctx.stroke()
                ctx.fillStyle = "rgba(220,225,232,0.6)"
                ctx.textAlign = "right"; ctx.fillText("0", x0 - 6, y0 + 3.5)
                ctx.textAlign = "left"; ctx.fillText("0", x1 + 6, y0 + 3.5)
            }
        }

        // The analyzer (-90..0 dB over the plot): after the EQ filled, before it as a line.
        Canvas {
            id: specCanvas
            x: graph.plotX; y: 0
            width: graph.plotW; height: graph.plotH
            visible: editor.analyzer > 0
            onPaint: {
                const ctx = getContext("2d")
                ctx.reset()
                const w = width, h = height
                const pts = sp => sp.map((v, i) => [(i + 0.5) / sp.length * w, h - Math.max(0, Math.min(1, (v + 90) / 96)) * h])
                const post = editor.specPost
                if (post.length > 1) {
                    const p = pts(post)
                    ctx.beginPath()
                    ctx.moveTo(0, h)
                    editor.spline(ctx, p, true)
                    ctx.lineTo(w, h)
                    ctx.closePath()
                    const g = ctx.createLinearGradient(0, 0, 0, h)
                    g.addColorStop(0, "rgba(140,190,255,0.30)")
                    g.addColorStop(0.6, "rgba(110,150,230,0.12)")
                    g.addColorStop(1, "rgba(90,120,200,0.02)")
                    ctx.fillStyle = g
                    ctx.fill()
                    ctx.beginPath()
                    editor.spline(ctx, p, false)
                    ctx.strokeStyle = "rgba(170,205,255,0.45)"
                    ctx.lineWidth = 1
                    ctx.stroke()
                }
                const pre = editor.specPre
                if (editor.analyzer === 2 && pre.length > 1) {
                    ctx.beginPath()
                    editor.spline(ctx, pts(pre), false)
                    ctx.strokeStyle = "rgba(255,255,255,0.22)"
                    ctx.lineWidth = 1
                    ctx.stroke()
                }
            }
        }

        // The selected band's shape, then the whole curve.
        Canvas {
            id: curveCanvas
            anchors.fill: parent
            onWidthChanged: requestPaint()
            onHeightChanged: requestPaint()
            onPaint: {
                const ctx = getContext("2d")
                ctx.reset()
                const y0 = graph.yOf(0)
                const toPts = c => c.map((db, i) => [graph.plotX + i / (c.length - 1) * graph.plotW, graph.yOf(Math.max(-graph.range - 6, Math.min(graph.range + 2, db)))])
                ctx.save()
                ctx.beginPath(); ctx.rect(graph.plotX, 0, graph.plotW, graph.plotH); ctx.clip()
                const sel = graph.selected >= 0 ? (graph.curves.bands || [])[graph.selected] : null
                if (sel && sel.length > 1) {
                    const col = graph.bands[graph.selected].color
                    ctx.beginPath()
                    ctx.moveTo(graph.plotX, y0)
                    editor.spline(ctx, toPts(sel), true)
                    ctx.lineTo(graph.plotX + graph.plotW, y0)
                    ctx.closePath()
                    ctx.fillStyle = editor.rgba(col, 0.22)
                    ctx.fill()
                    ctx.beginPath()
                    editor.spline(ctx, toPts(sel), false)
                    ctx.strokeStyle = editor.rgba(col, 0.75)
                    ctx.lineWidth = 1.2
                    ctx.stroke()
                }
                const total = graph.curves.total || []
                if (total.length > 1) {
                    const p = toPts(total)
                    // Soft fill between the curve and 0 dB.
                    ctx.beginPath()
                    ctx.moveTo(graph.plotX, y0)
                    editor.spline(ctx, p, true)
                    ctx.lineTo(graph.plotX + graph.plotW, y0)
                    ctx.closePath()
                    ctx.fillStyle = "rgba(255,255,255,0.05)"
                    ctx.fill()
                    ctx.lineJoin = "round"
                    ctx.lineCap = "round"
                    ctx.beginPath(); editor.spline(ctx, p, false)
                    ctx.strokeStyle = "rgba(255,255,255,0.12)"
                    ctx.lineWidth = 6
                    ctx.stroke()
                    ctx.beginPath(); editor.spline(ctx, p, false)
                    ctx.strokeStyle = "#F4F6F8"
                    ctx.lineWidth = 2.2
                    ctx.stroke()
                }
                ctx.restore()
            }
        }

        // Empty space: double-click adds the first free peaking band there.
        MouseArea {
            x: graph.plotX; y: 0
            width: graph.plotW; height: graph.plotH
            onDoubleClicked: m => {
                for (let i = 0; i < graph.bands.length; ++i) {
                    const b = graph.bands[i]
                    if (b.type === "Peaking" && editor.v(b.on) < 0.5) {
                        editor.set(b.freq, Math.round(graph.freqAt(m.x + graph.plotX)), true)
                        editor.set(b.gain, Math.round(graph.dbAt(m.y) * 10) / 10, true)
                        editor.set(b.on, 1, true)
                        graph.picked(i)
                        return
                    }
                }
            }
        }

        // Width handles for the selected peaking band or shelf.
        Item {
            id: qHandles
            readonly property var band: graph.selected >= 0 ? graph.bands[graph.selected] : null
            readonly property bool shown: !!band && !!band.q && editor.v(band.on) >= 0.5
            readonly property real f: band ? editor.v(band.freq) : 1000
            readonly property real half: band && band.q ? graph.octaves(editor.v(band.q)) / 2 : 0
            readonly property real cy: band ? graph.nodeY(band) : 0
            visible: shown
            anchors.fill: parent
            z: 2
            Repeater {
                model: [-1, 1]
                delegate: Rectangle {
                    required property int modelData
                    width: 8; height: 8; radius: 4
                    x: graph.xOf(qHandles.f * Math.pow(2, modelData * qHandles.half)) - width / 2
                    y: qHandles.cy - height / 2
                    color: "transparent"
                    border.color: qHandles.band ? qHandles.band.color : "white"
                    border.width: 1.5
                    scale: qArea.containsMouse || qArea.pressed ? 1.4 : 1
                    Behavior on scale { NumberAnimation { duration: Theme.fast } }
                    MouseArea {
                        id: qArea
                        anchors.fill: parent
                        anchors.margins: -6
                        hoverEnabled: true
                        preventStealing: true
                        cursorShape: Qt.SizeHorCursor
                        onPositionChanged: m => {
                            if (!pressed || !qHandles.band) return
                            const p = mapToItem(graph, m.x, m.y)
                            const oct = 2 * Math.abs(Math.log(graph.freqAt(p.x) / qHandles.f) / Math.LN2)
                            const d = editor.param(qHandles.band.q)
                            editor.set(qHandles.band.q, Math.max(d.min, Math.min(d.max, Math.round(graph.qFor(oct) * 100) / 100)), false)
                        }
                        onReleased: if (qHandles.band) editor.set(qHandles.band.q, editor.v(qHandles.band.q), true)
                    }
                    DToolTip { visible: qArea.containsMouse || qArea.pressed; delay: 0; text: qHandles.band && qHandles.band.q ? "Width · Q " + editor.v(qHandles.band.q).toFixed(2) : "" }
                }
            }
        }

        // Band handles: numbered circles in the band's colour.
        Repeater {
            model: graph.bands
            delegate: Item {
                id: node
                required property var modelData
                required property int index
                readonly property bool on: editor.v(modelData.on) >= 0.5
                readonly property bool current: graph.selected === index
                x: graph.xOf(editor.v(modelData.freq)) - width / 2
                y: graph.nodeY(modelData) - height / 2
                width: 30; height: 30
                z: current ? 4 : 3

                Rectangle {
                    anchors.centerIn: parent
                    width: 28; height: 28; radius: 14
                    color: Theme.tint(node.modelData.color, 0.18)
                    visible: node.current
                }
                Rectangle {
                    anchors.centerIn: parent
                    width: 18; height: 18; radius: 9
                    color: node.on ? node.modelData.color : "#1A1D22"
                    border.color: node.current ? "#FFFFFF" : node.on ? Qt.darker(node.modelData.color, 1.5) : node.modelData.color
                    border.width: node.current ? 2 : 1.5
                    opacity: node.on ? 1 : 0.6
                    scale: grab.containsMouse || grab.pressed ? 1.15 : 1
                    Behavior on scale { NumberAnimation { duration: Theme.fast; easing.type: Easing.OutCubic } }
                    DText {
                        anchors.centerIn: parent
                        text: node.index + 1
                        font.pixelSize: 10
                        font.weight: Font.Bold
                        color: node.on ? "#111317" : node.modelData.color
                    }
                }
                MouseArea {
                    id: grab
                    anchors.fill: parent
                    anchors.margins: 4
                    hoverEnabled: true
                    preventStealing: true
                    cursorShape: node.modelData.gain ? Qt.SizeAllCursor : Qt.SizeHorCursor
                    onPressed: graph.picked(node.index)
                    onPositionChanged: m => {
                        if (!pressed) return
                        const p = mapToItem(graph, m.x, m.y)
                        if (!node.on) editor.set(node.modelData.on, 1, false)
                        editor.set(node.modelData.freq, Math.round(graph.freqAt(p.x)), false)
                        if (node.modelData.gain) editor.set(node.modelData.gain, Math.round(graph.dbAt(p.y) * 10) / 10, false)
                    }
                    onReleased: {
                        const b = node.modelData
                        editor.set(b.on, editor.v(b.on), true)
                        editor.set(b.freq, editor.v(b.freq), true)
                        if (b.gain) editor.set(b.gain, editor.v(b.gain), true)
                    }
                    onDoubleClicked: editor.set(node.modelData.on, node.on ? 0 : 1, true)
                    onWheel: w => {
                        const b = node.modelData
                        const steps = w.angleDelta.y !== 0 ? w.angleDelta.y / 120 : w.pixelDelta.y / 30
                        if (b.q) {
                            const d = editor.param(b.q)
                            editor.set(b.q, Math.max(d.min, Math.min(d.max, editor.v(b.q) * Math.pow(1.15, steps))), true)
                        } else if (b.slope) {
                            editor.set(b.slope, Math.max(0, Math.min(3, Math.round(editor.v(b.slope) + Math.sign(steps)))), true)
                        }
                    }
                }
                // Live readout while dragging or hovering.
                Rectangle {
                    visible: grab.pressed || grab.containsMouse
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.bottom: parent.top
                    width: readout.implicitWidth + 12
                    height: 20
                    radius: 5
                    color: "#EE111317"
                    border.color: Theme.tint(node.modelData.color, 0.6)
                    DText {
                        id: readout
                        anchors.centerIn: parent
                        font.family: Theme.mono
                        font.pixelSize: 10
                        color: "#FFFFFF"
                        text: graph.fmtFreq(editor.v(node.modelData.freq))
                              + (node.modelData.gain ? "  " + (editor.v(node.modelData.gain) > 0 ? "+" : "") + editor.v(node.modelData.gain).toFixed(1) + " dB" : "")
                    }
                }
            }
        }

        // Top corners: analyzer mode and output level.
        Row {
            x: graph.plotX + 8; y: 8
            spacing: 6
            z: 5
            AbstractButton {
                id: presetButton
                implicitWidth: presetRow.implicitWidth + 20
                implicitHeight: 22
                hoverEnabled: true
                onClicked: presetPopup.opened ? presetPopup.close() : presetPopup.open()
                background: Rectangle { radius: 11; color: presetButton.hovered || presetPopup.opened ? "#2E333B" : "#1E2127"; border.color: "#343941" }
                contentItem: Item {
                    Row {
                        id: presetRow
                        anchors.centerIn: parent
                        spacing: 5
                        Icon { anchors.verticalCenter: parent.verticalCenter; glyph: Icons.sparkle; size: 11; color: "#C9CFD8" }
                        DText {
                            anchors.verticalCenter: parent.verticalCenter
                            text: editor.presetName ? editor.presetName + (editor.presetEdited ? " (edited)" : "") : "Presets"
                            font.pixelSize: 10
                            font.weight: Font.DemiBold
                            color: "#DDE2EA"
                        }
                        Icon { anchors.verticalCenter: parent.verticalCenter; glyph: Icons.caretDown; size: 9; color: "#8A9099" }
                    }
                }
                DToolTip { visible: presetButton.hovered && !presetPopup.opened; text: "Starting points for common sounds" }
                Popup {
                    id: presetPopup
                    y: parent.height + 6
                    padding: 14
                    modal: false
                    focus: true
                    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
                    background: Rectangle { radius: 12; color: "#F21A1D22"; border.color: "#343941" }
                    enter: Transition { NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.fast } }
                    exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: Theme.fast } }
                    contentItem: GridLayout {
                        columns: 3
                        columnSpacing: 18
                        rowSpacing: 12
                        Repeater {
                            model: editor.eqPresets
                            delegate: Column {
                                required property var modelData
                                Layout.alignment: Qt.AlignTop
                                spacing: 2
                                DText {
                                    text: modelData.group.toUpperCase()
                                    font.pixelSize: 9
                                    font.weight: Font.DemiBold
                                    font.letterSpacing: 1.2
                                    color: "#8A9099"
                                    bottomPadding: 4
                                }
                                Repeater {
                                    model: modelData.items
                                    delegate: AbstractButton {
                                        id: presetItem
                                        required property var modelData
                                        width: 168
                                        height: 26
                                        hoverEnabled: true
                                        readonly property bool current: editor.presetName === modelData.name
                                        onClicked: { editor.loadPreset(modelData); presetPopup.close() }
                                        background: Rectangle {
                                            radius: 6
                                            color: presetItem.current ? Theme.tint(Theme.accent, 0.25) : presetItem.hovered ? "#2A2E35" : "transparent"
                                        }
                                        contentItem: DText {
                                            leftPadding: 8
                                            verticalAlignment: Text.AlignVCenter
                                            text: presetItem.modelData.name
                                            font.pixelSize: 11
                                            color: presetItem.current ? "#FFFFFF" : "#D3D8E0"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            AbstractButton {
                id: anaButton
                implicitWidth: anaText.implicitWidth + 18
                implicitHeight: 22
                hoverEnabled: true
                onClicked: editor.analyzer = (editor.analyzer + 1) % 3
                background: Rectangle { radius: 11; color: anaButton.hovered ? "#2E333B" : "#1E2127"; border.color: "#343941" }
                contentItem: DText {
                    id: anaText
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    text: "Analyzer · " + ["Off", "Post", "Pre + Post"][editor.analyzer]
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    color: editor.analyzer ? "#DDE2EA" : "#6B7079"
                }
                DToolTip { visible: anaButton.hovered; text: "Spectrum: off, after the EQ, or before (line) and after (filled). Off saves a little CPU." }
            }
        }
        Row {
            anchors.right: parent.right; anchors.rightMargin: 38
            y: 7
            spacing: 6
            z: 5
            DText { anchors.verticalCenter: parent.verticalCenter; text: "Output"; font.pixelSize: 10; color: "#8A9099" }
            DragValue { key: "output" }
        }
    }

    // ================================================================ compressor
    // Styled like a hardware unit: a glowing amber gain-reduction meter with a
    // needle, big metal Threshold and Make-up knobs; beside it a second unit
    // with the transfer curve, the timing knobs, Mix, and hardware buttons.
    readonly property color amber: "#FF8A2A"

    Component {
        id: compBody
        RowLayout {
            spacing: 14

            // ---------------------------------------------- left unit
            Rectangle {
                Layout.preferredWidth: 380
                Layout.fillHeight: true
                radius: 18
                gradient: Gradient {
                    GradientStop { position: 0; color: "#26262A" }
                    GradientStop { position: 1; color: "#141416" }
                }
                border.color: "#34343A"

                VuScreen {
                    id: vu
                    x: 14; y: 14
                    width: parent.width - 28
                    height: Math.round(parent.height * 0.5)
                }
                Row {
                    anchors.horizontalCenter: parent.horizontalCenter
                    y: vu.y + vu.height + 14
                    spacing: 40
                    MetalKnob { key: "threshold"; label: "THRESHOLD"; size: 92 }
                    MetalKnob { key: "makeup"; label: "MAKE-UP"; size: 92 }
                }
                // Name plate and activity light.
                Rectangle {
                    anchors.bottom: parent.bottom
                    width: parent.width
                    height: 30
                    radius: 18
                    color: "#101012"
                    Rectangle { width: parent.width; height: 12; color: parent.color }
                    DText {
                        anchors.left: parent.left; anchors.leftMargin: 18
                        anchors.verticalCenter: parent.verticalCenter
                        text: "ISAI COMP"
                        font.family: Theme.mono
                        font.pixelSize: 11
                        font.letterSpacing: 2
                        color: "#9A9AA2"
                    }
                    Rectangle {
                        anchors.right: parent.right; anchors.rightMargin: 18
                        anchors.verticalCenter: parent.verticalCenter
                        width: 6; height: 6; radius: 3
                        color: editor.grHold < -0.5 ? editor.amber : "#3A2A1E"
                        Behavior on color { ColorAnimation { duration: 120 } }
                    }
                }
            }

            // ---------------------------------------------- right unit
            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: 18
                gradient: Gradient {
                    GradientStop { position: 0; color: "#2E2E33" }
                    GradientStop { position: 1; color: "#1C1C20" }
                }
                border.color: "#3A3A40"

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 14
                    spacing: 10
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        spacing: 12
                        TransferGraph { Layout.fillWidth: true; Layout.fillHeight: true }
                        MetalKnob { key: "mix"; label: "MIX"; size: 58; Layout.alignment: Qt.AlignTop }
                    }
                    Row {
                        Layout.alignment: Qt.AlignHCenter
                        spacing: 2
                        Repeater {
                            model: ["ratio", "attack", "release", "knee", "sc_hpf"]
                            delegate: ParamKnob {
                                required property string modelData
                                def: editor.param(modelData)
                                value: editor.v(modelData)
                                tint: editor.amber
                                onMoved: (v, done) => editor.set(modelData, v, done)
                            }
                        }
                    }
                    // Hardware-style keys.
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        HwKey {
                            label: editor.v("detect") >= 0.5 ? "RMS" : "PEAK"
                            lit: true
                            tip: "Detector: Peak reacts to every hit, RMS to the average loudness · click to switch"
                            onClicked: editor.set("detect", editor.v("detect") >= 0.5 ? 0 : 1, true)
                        }
                        HwKey {
                            label: "AUTO"
                            glyph: Icons.magicWand
                            lit: editor.v("auto_makeup") >= 0.5
                            tip: "Automatic make-up gain keeps the level about the same as you compress"
                            onClicked: editor.set("auto_makeup", lit ? 0 : 1, true)
                        }
                        HwKey {
                            glyph: Icons.lightning
                            lit: !!editor.insert && editor.insert.enabled
                            tip: lit ? "On · click to bypass" : "Bypassed · click to turn on"
                            onClicked: editor.app.toggleInsert(editor.insert.id)
                        }
                    }
                }
            }
        }
    }

    // The amber screen: a needle meter of gain reduction (0 at the right).
    component VuScreen: Rectangle {
        id: screen
        radius: 12
        color: "#0B0705"
        border.color: "#000000"
        clip: true
        readonly property real gr: Math.max(-30, Math.min(0, editor.grHold))
        /// 0 (left, -30 dB) .. 1 (right, 0 dB), stretched so small amounts read well.
        function pos(db) { return 1 - Math.pow(Math.min(30, Math.abs(db)) / 30, 0.55) }
        readonly property real sweep: 50
        readonly property real pivotX: width / 2
        readonly property real pivotY: height * 1.18
        readonly property real radius2: height * 0.98
        function dbText(v) { return (v > 0.05 ? "+" : "") + v.toFixed(1) + " dB" }

        // Glow, arc, scale: painted once per size.
        Canvas {
            anchors.fill: parent
            onWidthChanged: requestPaint()
            onHeightChanged: requestPaint()
            onPaint: {
                const ctx = getContext("2d")
                ctx.reset()
                const w = width, h = height
                const glow = ctx.createRadialGradient(w / 2, h * 0.62, 4, w / 2, h * 0.62, w * 0.62)
                glow.addColorStop(0, "rgba(255,120,30,0.42)")
                glow.addColorStop(0.45, "rgba(255,90,20,0.14)")
                glow.addColorStop(1, "rgba(0,0,0,0)")
                ctx.fillStyle = glow
                ctx.fillRect(0, 0, w, h)
                const cx = screen.pivotX, cy = screen.pivotY, r = screen.radius2
                const ang = p => (-90 - screen.sweep + 2 * screen.sweep * p) * Math.PI / 180
                ctx.strokeStyle = "rgba(255,138,42,0.75)"
                ctx.lineWidth = 1.5
                ctx.beginPath(); ctx.arc(cx, cy, r * 0.86, ang(0), ang(1)); ctx.stroke()
                ctx.font = "500 12px monospace"
                ctx.textAlign = "center"
                ctx.fillStyle = "rgba(255,200,160,0.9)"
                for (const d of [-30, -20, -10, -5, -3, -1, 0]) {
                    const a = ang(screen.pos(d))
                    const major = [-30, -20, -10, -5, 0].indexOf(d) >= 0
                    const r1 = r * 0.86, r2 = r * (major ? 0.94 : 0.91)
                    ctx.strokeStyle = "rgba(255,138,42,0.85)"
                    ctx.beginPath()
                    ctx.moveTo(cx + Math.cos(a) * r1, cy + Math.sin(a) * r1)
                    ctx.lineTo(cx + Math.cos(a) * r2, cy + Math.sin(a) * r2)
                    ctx.stroke()
                    if (major) ctx.fillText(String(d), cx + Math.cos(a) * r * 1.03, cy + Math.sin(a) * r * 1.03 + 4)
                }
            }
        }

        // The needle (eased, like a real meter's ballistics).
        Item {
            x: screen.pivotX - 1
            y: screen.pivotY - screen.radius2 * 0.95
            width: 2
            height: screen.radius2 * 0.95
            transformOrigin: Item.Bottom
            rotation: -screen.sweep + 2 * screen.sweep * screen.pos(screen.gr)
            Behavior on rotation { SmoothedAnimation { velocity: 220 } }
            Rectangle { anchors.horizontalCenter: parent.horizontalCenter; width: 8; height: parent.height; radius: 4; color: Theme.tint(editor.amber, 0.18) }
            Rectangle { anchors.horizontalCenter: parent.horizontalCenter; width: 2; height: parent.height; radius: 1; color: "#FFB066" }
        }

        DText {
            x: 12; y: 10
            text: "GAIN REDUCTION"
            font.family: Theme.mono
            font.pixelSize: 10
            font.letterSpacing: 1
            color: Theme.tint(editor.amber, 0.85)
        }
        DText {
            anchors.right: parent.right; anchors.rightMargin: 12; y: 10
            text: editor.v("detect") >= 0.5 ? "RMS" : "PEAK"
            font.family: Theme.mono
            font.pixelSize: 10
            color: Theme.tint(editor.amber, 0.6)
        }
        Column {
            anchors.horizontalCenter: parent.horizontalCenter
            y: parent.height * 0.55
            spacing: 0
            DText {
                anchors.horizontalCenter: parent.horizontalCenter
                text: "COMPRESSOR"
                font.family: Theme.mono
                font.pixelSize: 17
                font.weight: Font.DemiBold
                font.letterSpacing: 3
                color: "#FFB066"
                style: Text.Raised
                styleColor: Theme.tint(editor.amber, 0.45)
            }
            DText {
                anchors.horizontalCenter: parent.horizontalCenter
                text: screen.gr.toFixed(1) + " dB"
                font.family: Theme.mono
                font.pixelSize: 11
                color: Theme.tint("#FFB066", 0.8)
            }
        }
        Column {
            x: 12; anchors.bottom: parent.bottom; anchors.bottomMargin: 10
            DText { text: "INPUT"; font.family: Theme.mono; font.pixelSize: 9; color: Theme.tint(editor.amber, 0.7) }
            DText {
                readonly property real lv: Math.max(editor.meters.in ? editor.meters.in[0] : 0, editor.meters.in ? editor.meters.in[1] : 0)
                text: lv > 0 ? screen.dbText(lv * 60 - 60) : "—"
                font.family: Theme.mono; font.pixelSize: 11; color: "#FFB066"
            }
        }
        Column {
            anchors.right: parent.right; anchors.rightMargin: 12
            anchors.bottom: parent.bottom; anchors.bottomMargin: 10
            DText { anchors.right: parent.right; text: "OUTPUT"; font.family: Theme.mono; font.pixelSize: 9; color: Theme.tint(editor.amber, 0.7) }
            DText {
                anchors.right: parent.right
                readonly property real lv: Math.max(editor.meters.out ? editor.meters.out[0] : 0, editor.meters.out ? editor.meters.out[1] : 0)
                text: lv > 0 ? screen.dbText(lv * 60 - 60) : "—"
                font.family: Theme.mono; font.pixelSize: 11; color: "#FFB066"
            }
        }
    }

    // A big brushed-metal knob for one parameter. Drag up/down (⇧ fine),
    // scroll, double-click for the default.
    component MetalKnob: Item {
        id: mk
        property string key
        property string label
        property real size: 90
        readonly property var def: editor.param(key)
        readonly property real value: editor.v(key)
        function posOf(v) {
            if (!def) return 0
            const x = Math.max(def.min, Math.min(def.max, v))
            if (def.log && def.min > 0) return Math.log(x / def.min) / Math.log(def.max / def.min)
            return (x - def.min) / (def.max - def.min)
        }
        function valueOf(p) {
            p = Math.max(0, Math.min(1, p))
            if (def.log && def.min > 0) return def.min * Math.pow(def.max / def.min, p)
            return def.min + p * (def.max - def.min)
        }
        function fmt(v) {
            if (!def) return ""
            switch (def.unit) {
            case "dB": return (v > 0.05 ? "+" : "") + v.toFixed(1) + " dB"
            case "%": return Math.round(v) + "%"
            case "ms": return v < 10 ? v.toFixed(1) + " ms" : Math.round(v) + " ms"
            default: return v.toFixed(1) + (def.unit ? " " + def.unit : "")
            }
        }
        readonly property real pos: posOf(value)
        implicitWidth: size + 24
        implicitHeight: size + 40

        // Well with tick dots (painted once).
        Canvas {
            id: well
            width: mk.size + 24; height: mk.size + 24
            onPaint: {
                const ctx = getContext("2d")
                ctx.reset()
                const c = width / 2, r = mk.size / 2
                ctx.fillStyle = "rgba(0,0,0,0.45)"
                ctx.beginPath(); ctx.arc(c, c, r + 3, 0, Math.PI * 2); ctx.fill()
                ctx.fillStyle = "rgba(255,255,255,0.45)"
                for (let i = 0; i <= 10; ++i) {
                    const a = (135 + 27 * i) * Math.PI / 180
                    ctx.beginPath(); ctx.arc(c + Math.cos(a) * (r + 9), c + Math.sin(a) * (r + 9), i % 5 === 0 ? 1.6 : 1, 0, Math.PI * 2); ctx.fill()
                }
            }
        }
        // Brushed metal cap (painted once), with the pointer turning on top.
        Canvas {
            id: cap
            x: 12; y: 12
            width: mk.size; height: mk.size
            onPaint: {
                const ctx = getContext("2d")
                ctx.reset()
                const c = width / 2, r = width / 2 - 1
                const g = ctx.createConicalGradient(c, c, 0)
                const stops = ["#5A5A60", "#C8C8CE", "#6A6A70", "#2E2E33", "#B5B5BC", "#55555B", "#D2D2D8", "#4A4A50", "#5A5A60"]
                stops.forEach((s, i) => g.addColorStop(i / (stops.length - 1), s))
                ctx.fillStyle = g
                ctx.beginPath(); ctx.arc(c, c, r, 0, Math.PI * 2); ctx.fill()
                const shade = ctx.createRadialGradient(c * 0.7, c * 0.6, 2, c, c, r)
                shade.addColorStop(0, "rgba(255,255,255,0.18)")
                shade.addColorStop(1, "rgba(0,0,0,0.35)")
                ctx.fillStyle = shade
                ctx.beginPath(); ctx.arc(c, c, r, 0, Math.PI * 2); ctx.fill()
                ctx.strokeStyle = "rgba(0,0,0,0.6)"
                ctx.lineWidth = 1.5
                ctx.beginPath(); ctx.arc(c, c, r * 0.62, 0, Math.PI * 2); ctx.stroke()
                ctx.strokeStyle = "rgba(255,255,255,0.12)"
                ctx.beginPath(); ctx.arc(c, c, r * 0.62 + 1.5, 0, Math.PI * 2); ctx.stroke()
            }
            Item {
                anchors.fill: parent
                rotation: -135 + 270 * mk.pos
                Behavior on rotation { enabled: !mkArea.pressed; NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    y: 6
                    width: 3; height: mk.size * 0.2
                    radius: 1.5
                    color: "#1A1A1C"
                }
            }
        }
        DText {
            anchors.horizontalCenter: parent.horizontalCenter
            y: mk.size + 22
            text: mkArea.pressed || mkArea.containsMouse ? mk.fmt(mk.value) : mk.label
            font.family: Theme.mono
            font.pixelSize: 10
            font.letterSpacing: 1.5
            color: mkArea.pressed ? editor.amber : "#B5B5BC"
        }
        MouseArea {
            id: mkArea
            x: 12; y: 12
            width: mk.size; height: mk.size
            hoverEnabled: true
            preventStealing: true
            cursorShape: Qt.SizeVerCursor
            property real startY
            property real startPos
            onPressed: m => { startY = m.y; startPos = mk.pos }
            onPositionChanged: m => {
                if (!pressed || !mk.def) return
                const fine = m.modifiers & Qt.ShiftModifier ? 0.25 : 1
                editor.set(mk.key, mk.valueOf(startPos + (startY - m.y) / 220 * fine), false)
            }
            onReleased: editor.set(mk.key, mk.value, true)
            onDoubleClicked: if (mk.def) editor.set(mk.key, mk.def.default, true)
            onWheel: w => {
                if (!mk.def) return
                const steps = w.angleDelta.y !== 0 ? w.angleDelta.y / 120 : w.pixelDelta.y / 30
                editor.set(mk.key, mk.valueOf(mk.pos + steps * 0.02), true)
            }
        }
    }

    // A chunky hardware key that glows amber when on.
    component HwKey: AbstractButton {
        id: key
        property string label: ""
        property string glyph: ""
        property bool lit: false
        property string tip: ""
        Layout.fillWidth: true
        implicitHeight: 46
        hoverEnabled: true
        scale: down ? 0.96 : 1
        Behavior on scale { NumberAnimation { duration: 80 } }
        background: Rectangle {
            radius: 10
            gradient: Gradient {
                GradientStop { position: 0; color: key.hovered ? "#2C2C31" : "#252529" }
                GradientStop { position: 1; color: "#141416" }
            }
            border.color: key.lit ? Theme.tint(editor.amber, 0.55) : "#3A3A40"
            Rectangle {
                anchors.fill: parent
                anchors.margins: 1
                radius: 9
                color: "transparent"
                border.color: Theme.tint(editor.amber, key.lit ? 0.18 : 0)
                border.width: 3
            }
        }
        contentItem: Row {
            spacing: 6
            leftPadding: (key.width - implicitWidth) / 2
            Icon {
                visible: key.glyph !== ""
                anchors.verticalCenter: parent.verticalCenter
                glyph: key.glyph
                size: 15
                color: key.lit ? "#FFB066" : "#8A8A92"
            }
            DText {
                visible: key.label !== ""
                anchors.verticalCenter: parent.verticalCenter
                text: key.label
                font.family: Theme.mono
                font.pixelSize: 12
                font.letterSpacing: 1.5
                color: key.lit ? "#FFB066" : "#B5B5BC"
            }
        }
        DToolTip { visible: key.hovered && key.tip !== ""; text: key.tip }
    }

    component TransferGraph: Item {
        id: tg
        readonly property real th: editor.v("threshold")
        readonly property real ratio: editor.v("ratio")
        readonly property real knee: editor.v("knee")
        function curve(x) {
            const over = x - th, slope = 1 / Math.max(1, ratio) - 1
            if (knee > 0 && Math.abs(over) <= knee / 2) return x + slope * Math.pow(over + knee / 2, 2) / (2 * knee)
            return over > 0 ? x + slope * over : x
        }
        // The live input level (meters are 0..1 on a -60..0 dB scale).
        readonly property real inputDb: Math.max(editor.meters.in ? editor.meters.in[0] : 0, editor.meters.in ? editor.meters.in[1] : 0) * 60 - 60
        onThChanged: c.requestPaint()
        onRatioChanged: c.requestPaint()
        onKneeChanged: c.requestPaint()
        onWidthChanged: c.requestPaint()
        onHeightChanged: c.requestPaint()
        Rectangle {
            anchors.fill: parent
            radius: 10
            color: "#0D0D0F"
            border.color: "#000000"
        }
        // Dotted grille behind the curve, like a speaker cover.
        Canvas {
            id: c
            anchors.fill: parent
            anchors.margins: 1
            onPaint: {
                const ctx = getContext("2d")
                ctx.reset()
                const w = width, h = height
                ctx.fillStyle = "rgba(255,255,255,0.07)"
                for (let y = 7; y < h; y += 9)
                    for (let x = 7 + ((y / 9) % 2) * 4.5; x < w; x += 9) {
                        ctx.beginPath(); ctx.arc(x, y, 1.3, 0, Math.PI * 2); ctx.fill()
                    }
                const X = d => (d + 60) / 60 * w, Y = d => h - (d + 60) / 60 * h
                ctx.strokeStyle = "rgba(255,255,255,0.15)"
                ctx.setLineDash([3, 3])
                ctx.beginPath(); ctx.moveTo(0, h); ctx.lineTo(w, 0); ctx.stroke()
                ctx.setLineDash([])
                ctx.strokeStyle = editor.rgba(editor.amber, 0.45)
                ctx.beginPath(); ctx.moveTo(X(tg.th), 0); ctx.lineTo(X(tg.th), h); ctx.stroke()
                ctx.beginPath()
                for (let i = 0; i <= 120; ++i) {
                    const x = -60 + i * 0.5
                    if (i === 0) ctx.moveTo(X(x), Y(tg.curve(x))); else ctx.lineTo(X(x), Y(tg.curve(x)))
                }
                ctx.strokeStyle = editor.rgba(editor.amber, 0.25)
                ctx.lineWidth = 7
                ctx.stroke()
                ctx.strokeStyle = "#FFB066"
                ctx.lineWidth = 2.2
                ctx.stroke()
            }
        }
        // Where the signal is now (an item, so the curve isn't repainted each frame).
        Rectangle {
            readonly property real w: c.width
            readonly property real h: c.height
            visible: tg.inputDb > -59
            width: 10; height: 10; radius: 5
            color: "#FFD2A8"
            border.color: editor.amber
            border.width: 2
            x: 1 + (tg.inputDb + 60) / 60 * w - 5
            y: 1 + h - (tg.curve(tg.inputDb) + 60) / 60 * h - 5
            Behavior on x { NumberAnimation { duration: 60 } }
            Behavior on y { NumberAnimation { duration: 60 } }
        }
        DText {
            anchors.left: parent.left; anchors.top: parent.top; anchors.margins: 8
            text: "THRESHOLD " + tg.th.toFixed(1) + " dB  ·  " + tg.ratio.toFixed(1) + ":1"
            font.family: Theme.mono
            font.pixelSize: 10
            color: Theme.tint("#FFB066", 0.85)
        }
    }

    // ================================================================ limiter
    Component {
        id: limiterBody
        RowLayout {
            spacing: 14
            ReductionMeter { Layout.fillHeight: true }
            KnobGrid {
                Layout.fillWidth: true
                Layout.fillHeight: true
                columns: 3
                big: true
            }
        }
    }

    // ================================================================ others
    Component {
        id: knobsBody
        KnobGrid { columns: 4; big: true }
    }

    // Every parameter of the effect as knobs.
    component KnobGrid: Item {
        id: kg
        property int columns: 4
        property bool big: false
        GridLayout {
            anchors.centerIn: parent
            width: Math.min(parent.width, kg.columns * (kg.big ? 96 : 76))
            columns: kg.columns
            rowSpacing: 10
            columnSpacing: 0
            Repeater {
                model: editor.def ? editor.def.params : []
                delegate: ParamKnob {
                    required property var modelData
                    Layout.alignment: Qt.AlignHCenter
                    scale: kg.big ? 1.25 : 1.1
                    Layout.preferredHeight: kg.big ? 84 : 72
                    def: modelData
                    value: editor.v(modelData.key)
                    tint: editor.tint
                    onMoved: (v, done) => editor.set(modelData.key, v, done)
                }
            }
        }
    }

    // How much the effect is turning the sound down right now.
    component ReductionMeter: Item {
        implicitWidth: 44
        readonly property real gr: Math.min(0, editor.grHold)
        Rectangle {
            id: track
            anchors.horizontalCenter: parent.horizontalCenter
            y: 18
            width: 14
            height: parent.height - 40
            radius: 4
            color: Theme.tint(Theme.sunken, 0.8)
            border.color: Theme.line
            Rectangle {
                width: parent.width
                height: parent.height * Math.min(1, -parent.parent.gr / 24)
                radius: 4
                color: Theme.warning
            }
        }
        DText {
            anchors.horizontalCenter: parent.horizontalCenter
            text: "GR"
            font.pixelSize: 10
            faint: true
        }
        DText {
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            text: parent.gr.toFixed(1)
            font.family: Theme.mono
            font.pixelSize: 10
            dim: true
        }
    }

    // A pair of vertical level bars.
    component StereoMeter: Column {
        property string label
        property var levels: [0, 0]
        spacing: 6
        DText { anchors.horizontalCenter: parent.horizontalCenter; text: parent.label; font.pixelSize: 10; faint: true }
        Row {
            spacing: 3
            height: parent.height - 20
            Repeater {
                model: 2
                delegate: Rectangle {
                    required property int index
                    width: 7
                    height: parent.height
                    radius: 3
                    color: Theme.tint(Theme.sunken, 0.8)
                    border.color: Theme.line
                    Rectangle {
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: parent.height * Math.max(0, Math.min(1, parent.parent.parent.levels[parent.index] || 0))
                        radius: 3
                        gradient: Gradient {
                            GradientStop { position: 0.0; color: Theme.danger }
                            GradientStop { position: 0.15; color: Theme.warning }
                            GradientStop { position: 0.3; color: Theme.success }
                            GradientStop { position: 1.0; color: Theme.success }
                        }
                    }
                }
            }
        }
    }
}

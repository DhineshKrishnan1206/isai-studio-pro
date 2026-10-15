pragma Singleton
import QtQuick

// The song's bars and tempos over time, for drawing. Main.qml feeds it
// `app.tempoJson`. Positions are in the song's beats (its first time
// signature's note value), like everything else on the timeline.
//
//   sigs:   [[first beat, first bar, numerator, denominator]…], from bar 1
//   tempos: [[first beat, bpm]…], from beat 0
QtObject {
    property string json: ""
    readonly property var data: {
        try { return JSON.parse(json || "{}") } catch (e) { return {} }
    }
    readonly property var sigs: data.sigs && data.sigs.length ? data.sigs : [[0, 1, 4, 4]]
    readonly property var tempos: data.tempos && data.tempos.length ? data.tempos : [[0, 120]]
    readonly property int baseDen: sigs[0][3]
    /// Any tempo or time-signature changes after the start.
    readonly property bool changes: sigs.length > 1 || tempos.length > 1

    function sectionAt(beat) {
        let s = sigs[0]
        for (let i = 1; i < sigs.length && sigs[i][0] <= beat + 1e-9; i++) s = sigs[i]
        return s
    }
    /// One beat of a section (its own note value), in song beats.
    function unitOf(s) { return baseDen / s[3] }
    function barLenOf(s) { return s[2] * unitOf(s) }
    /// The bar length (in beats) where a position is.
    function barLenAt(beat) { return barLenOf(sectionAt(beat)) }

    /// Where a bar (counted from 1) starts.
    function barStart(bar) {
        let s = sigs[0]
        for (let i = 1; i < sigs.length && sigs[i][1] <= bar; i++) s = sigs[i]
        return s[0] + (bar - s[1]) * barLenOf(s)
    }
    /// The bar (counted from 1) a position is in.
    function barAt(beat) {
        const s = sectionAt(Math.max(0, beat))
        return s[1] + Math.floor((Math.max(0, beat) - s[0]) / barLenOf(s) + 1e-9)
    }
    function isBarLine(beat) {
        const s = sectionAt(beat)
        const k = (beat - s[0]) / barLenOf(s)
        return Math.abs(k - Math.round(k)) < 1e-6
    }

    /// The bars that start between two positions (and the one before `from`):
    /// [{ bar, beat, num, den, unit, len }…]. At most `limit` of them.
    function barsBetween(from, to, limit) {
        const out = []
        let bar = barAt(from)
        let beat = barStart(bar)
        while (beat <= to && out.length < (limit || 2000)) {
            const s = sectionAt(beat)
            const unit = unitOf(s)
            out.push({ bar: bar, beat: beat, num: s[2], den: s[3], unit: unit, len: s[2] * unit })
            bar += 1
            beat = barStart(bar)
        }
        return out
    }

    function bpmAt(beat) {
        let t = tempos[0]
        for (let i = 1; i < tempos.length && tempos[i][0] <= beat + 1e-9; i++) t = tempos[i]
        return t[1]
    }
}

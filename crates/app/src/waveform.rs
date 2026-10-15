//! A QML item that draws one region's slice of an audio file with QPainter.
//!
//! It only repaints when its source, slice, size, colour or detail changes,
//! never during playback (the playhead is a separate QML item).

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qcolor.h");
        type QColor = cxx_qt_lib::QColor;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qrectf.h");
        type QRectF = cxx_qt_lib::QRectF;
        include!("cxx-qt-lib/qsizef.h");
        type QSizeF = cxx_qt_lib::QSizeF;
        include!("cxx-qt-lib/qpainter.h");
        type QPainter = cxx_qt_lib::QPainter;
    }

    unsafe extern "C++" {
        include!(<QtQuick/QQuickPaintedItem>);
        type QQuickPaintedItem;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[base = QQuickPaintedItem]
        #[qproperty(QColor, color)]
        /// Width of each drawn column in pixels: 1 is most detailed, 4 is lightest.
        #[qproperty(i32, column_width)]
        /// The audio file's path (the key its waveform summary is stored under).
        #[qproperty(QString, source)]
        /// The slice of the file to draw, in seconds.
        #[qproperty(f64, offset)]
        #[qproperty(f64, length)]
        /// Linear gain to draw with (the region's volume change).
        #[qproperty(f64, gain)]
        /// Draw the slice backwards, as a reversed region plays it.
        #[qproperty(bool, reverse)]
        type WaveformView = super::WaveformViewRust;

        #[qinvokable]
        #[cxx_override]
        unsafe fn paint(self: Pin<&mut Self>, painter: *mut QPainter);

        #[inherit]
        fn size(self: &Self) -> QSizeF;

        #[inherit]
        fn update(self: Pin<&mut Self>);
    }

    impl cxx_qt::Initialize for WaveformView {}
}

use core::pin::Pin;

use cxx_qt_lib::{QColor, QRectF, QString};

use crate::shared;

pub struct WaveformViewRust {
    color: QColor,
    column_width: i32,
    source: QString,
    offset: f64,
    length: f64,
    gain: f64,
    reverse: bool,
}

impl Default for WaveformViewRust {
    fn default() -> Self {
        Self {
            color: QColor::from_rgb(123, 108, 255),
            column_width: 2,
            source: QString::default(),
            offset: 0.0,
            length: 0.0,
            gain: 1.0,
            reverse: false,
        }
    }
}

impl qobject::WaveformView {
    /// # Safety
    /// Qt passes a valid painter that lives for the duration of this call.
    pub unsafe fn paint(self: Pin<&mut Self>, painter: *mut qobject::QPainter) {
        // SAFETY: see above; the pointer is valid and not aliased during paint.
        let Some(painter) = (unsafe { painter.as_mut() }) else { return };
        // SAFETY: the painter is not moved while pinned.
        let mut painter = unsafe { Pin::new_unchecked(painter) };
        let Some(wave) = shared::peaks(&self.source().to_string()) else { return };

        let size = self.size();
        let (width, height) = (size.width(), size.height());
        let all = &wave.peaks.values;
        // The peaks covering this region's slice of the file.
        let per_sec = f64::from(wave.sample_rate) / wave.peaks.frames_per_peak as f64;
        let first = ((*self.offset() * per_sec) as usize).min(all.len());
        let last = (((*self.offset() + *self.length()) * per_sec).ceil() as usize).clamp(first, all.len());
        let peaks = &all[first..last];
        if width <= 0.0 || peaks.is_empty() {
            return;
        }

        let color = self.color().clone();
        let step = f64::from((*self.column_width()).max(1));
        // Leave a hairline gap between columns at low detail so bars read clearly.
        let bar = if step >= 2.0 { step - 0.5 } else { step };
        let mid = height / 2.0;
        let half = height * 0.46;
        let gain = *self.gain() as f32;
        let reverse = *self.reverse();
        let peaks_per_px = peaks.len() as f64 / width;

        let mut x = 0.0;
        while x < width {
            // A reversed region reads its peaks from the far end.
            let at = if reverse { (width - x - step).max(0.0) } else { x };
            let first = ((at * peaks_per_px) as usize).min(peaks.len() - 1);
            let last = (((at + step) * peaks_per_px) as usize).clamp(first + 1, peaks.len());
            let (lo, hi) = peaks[first..last]
                .iter()
                .fold((0.0f32, 0.0f32), |(lo, hi), &(a, b)| (lo.min(a), hi.max(b)));
            let (lo, hi) = ((lo * gain).max(-1.0), (hi * gain).min(1.0));
            let top = mid - f64::from(hi) * half;
            // At least one pixel tall, so silence still shows as a thin line.
            let h = (f64::from(hi - lo) * half).max(1.0);
            painter.as_mut().fill_rect(&QRectF::new(x, top, bar, h), &color);
            x += step;
        }
    }
}

impl cxx_qt::Initialize for qobject::WaveformView {
    fn initialize(mut self: Pin<&mut Self>) {
        self.as_mut().on_color_changed(|item| item.update()).release();
        self.as_mut().on_column_width_changed(|item| item.update()).release();
        self.as_mut().on_source_changed(|item| item.update()).release();
        self.as_mut().on_offset_changed(|item| item.update()).release();
        self.as_mut().on_length_changed(|item| item.update()).release();
        self.as_mut().on_gain_changed(|item| item.update()).release();
        self.as_mut().on_reverse_changed(|item| item.update()).release();
    }
}

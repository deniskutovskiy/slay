use crate::theme::*;
use eframe::egui;
use slay_core::MetricPoint;

pub struct SparklineWidget<'a> {
    label: String,
    data: &'a [MetricPoint],
    field_extractor: Box<dyn Fn(&MetricPoint) -> f32 + 'a>,
    color: egui::Color32,
    current_value_text: String,
    size: egui::Vec2,
}

impl<'a> SparklineWidget<'a> {
    pub fn new(
        label: &str,
        data: &'a [MetricPoint],
        extractor: impl Fn(&MetricPoint) -> f32 + 'a,
        color: egui::Color32,
        value_text: String,
    ) -> Self {
        Self {
            label: label.to_string(),
            data,
            field_extractor: Box::new(extractor),
            color,
            current_value_text: value_text,
            size: egui::vec2(180.0, 45.0),
        }
    }
}

impl egui::Widget for SparklineWidget<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(self.size, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            painter.rect_filled(rect, 2.0, egui::Color32::from_black_alpha(40));
            painter.rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
            );

            // Text Area (Top 15 pixels reserved for text)
            let text_margin_y = 18.0;
            let graph_rect = egui::Rect::from_min_max(
                rect.left_top() + egui::vec2(0.0, text_margin_y),
                rect.right_bottom(),
            );

            if self.data.len() >= 2 {
                let values: Vec<f32> = self
                    .data
                    .iter()
                    .map(|p| (self.field_extractor)(p))
                    .collect();
                let max_val = values.iter().copied().fold(0.0, f32::max).max(0.001);

                let points: Vec<egui::Pos2> = values
                    .iter()
                    .enumerate()
                    .map(|(i, &v)| {
                        let x = graph_rect.left()
                            + (i as f32 / (values.len() - 1) as f32) * graph_rect.width();
                        let y =
                            graph_rect.bottom() - (v / max_val) * (graph_rect.height() * 0.8) - 2.0;
                        egui::pos2(x, y)
                    })
                    .collect();

                // Area Fill
                let mut shape_points = points.clone();
                shape_points.push(egui::pos2(graph_rect.right(), graph_rect.bottom()));
                shape_points.push(egui::pos2(graph_rect.left(), graph_rect.bottom()));
                painter.add(egui::Shape::convex_polygon(
                    shape_points,
                    self.color.gamma_multiply(0.15),
                    egui::Stroke::NONE,
                ));

                // Line
                painter.add(egui::Shape::line(
                    points,
                    egui::Stroke::new(1.5, self.color),
                ));
            } else {
                painter.text(
                    graph_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "ANALYZING...",
                    egui::FontId::proportional(10.0),
                    COLOR_TEXT_DIM,
                );
            }

            // Labels - Placed in the reserved top area
            painter.text(
                rect.left_top() + egui::vec2(8.0, 4.0),
                egui::Align2::LEFT_TOP,
                &self.label,
                egui::FontId::proportional(10.0),
                COLOR_TEXT_DIM,
            );
            painter.text(
                rect.right_top() + egui::vec2(-8.0, 4.0),
                egui::Align2::RIGHT_TOP,
                &self.current_value_text,
                egui::FontId::proportional(13.0),
                self.color,
            );
        }

        response
    }
}

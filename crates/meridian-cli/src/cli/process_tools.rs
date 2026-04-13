use meridian_core::midi::{
    MidiFileProcessingConfig, MidiModifierTool, QuantizeMode, QuantizeTool, RangeEdgeBehavior,
    RangeSelectTool, TempoMapTool, analysis::MidiAnalysisKind,
};

use super::args::{
    AnalysisKindArg, AnalyzeArgs, ProcessQuantizeArgs, ProcessSelectArgs, ProcessTempoFlattenArgs,
    ProcessTempoScaleArgs, QuantizeModeArg, RangeEdgeBehaviorArg,
};

pub(super) enum ProcessTool {
    Select(RangeSelectTool),
    Tempo(TempoMapTool),
    Quantize(QuantizeTool),
}

pub(super) fn analysis_kinds(args: &AnalyzeArgs) -> Vec<MidiAnalysisKind> {
    let mut kinds = if args.include.is_empty() {
        default_analysis_kinds()
    } else {
        args.include
            .iter()
            .copied()
            .map(map_analysis_kind)
            .collect()
    };
    if args.buckets.is_some() {
        kinds.push(MidiAnalysisKind::Buckets);
    }
    kinds
}

pub(super) fn build_process_config(tool: ProcessTool) -> MidiFileProcessingConfig {
    MidiFileProcessingConfig {
        tool: match tool {
            ProcessTool::Select(tool) => MidiModifierTool::RangeSelect(tool),
            ProcessTool::Tempo(tool) => MidiModifierTool::TempoMap(tool),
            ProcessTool::Quantize(tool) => MidiModifierTool::Quantize(tool),
        },
    }
}

pub(super) fn process_range_select_tool(args: &ProcessSelectArgs) -> RangeSelectTool {
    RangeSelectTool {
        start_ticks: args.start_ticks,
        end_ticks: args.end_ticks,
        offset_ticks: args.offset_ticks,
        track_select: args.track_select,
        preserve_system_events: args.preserve_system_events,
        edge_behavior: map_range_edge_behavior(args.edge_behavior),
    }
}

pub(super) fn process_tempo_flatten_tool(args: &ProcessTempoFlattenArgs) -> TempoMapTool {
    TempoMapTool::Flatten { tempo: args.tempo }
}

pub(super) fn process_tempo_scale_tool(args: &ProcessTempoScaleArgs) -> TempoMapTool {
    TempoMapTool::ScaleBpm {
        factor: args.factor,
    }
}

pub(super) fn process_quantize_tool(args: &ProcessQuantizeArgs) -> QuantizeTool {
    QuantizeTool {
        rounding_ticks: args.grid_ticks,
        mode: map_quantize_mode(args.mode),
    }
}

fn default_analysis_kinds() -> Vec<MidiAnalysisKind> {
    vec![
        MidiAnalysisKind::File,
        MidiAnalysisKind::Summary,
        MidiAnalysisKind::Events,
        MidiAnalysisKind::Notes,
        MidiAnalysisKind::Tempo,
    ]
}

fn map_analysis_kind(kind: AnalysisKindArg) -> MidiAnalysisKind {
    match kind {
        AnalysisKindArg::File => MidiAnalysisKind::File,
        AnalysisKindArg::Summary => MidiAnalysisKind::Summary,
        AnalysisKindArg::Events => MidiAnalysisKind::Events,
        AnalysisKindArg::Notes => MidiAnalysisKind::Notes,
        AnalysisKindArg::Tempo => MidiAnalysisKind::Tempo,
    }
}

fn map_range_edge_behavior(kind: RangeEdgeBehaviorArg) -> RangeEdgeBehavior {
    match kind {
        RangeEdgeBehaviorArg::Keep => RangeEdgeBehavior::Keep,
        RangeEdgeBehaviorArg::Skip => RangeEdgeBehavior::Skip,
        RangeEdgeBehaviorArg::Trim => RangeEdgeBehavior::Trim,
    }
}

fn map_quantize_mode(kind: QuantizeModeArg) -> QuantizeMode {
    match kind {
        QuantizeModeArg::NoteStartOnly => QuantizeMode::NoteStartOnly,
        QuantizeModeArg::NoteStartAndEnd => QuantizeMode::NoteStartAndEnd,
        QuantizeModeArg::AllEvents => QuantizeMode::AllEvents,
    }
}

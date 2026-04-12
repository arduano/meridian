use super::*;

pub(in super::super) fn csv_parts<'a>(raw: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
}

pub(in super::super) fn parse_u8_token(raw: &str, label: &str) -> Result<u8, String> {
    let trimmed = raw.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u8::from_str_radix(hex, 16).map_err(|error| format!("invalid {label}: {error}"))
    } else {
        parse_value(trimmed, label)
    }
}

pub(in super::super) fn parse_arrow_entries<T>(
    raw: &str,
    label: &str,
) -> Result<Vec<(T, T)>, String>
where
    T: FromStr,
    T::Err: Display,
{
    let mut result = Vec::new();
    for entry in csv_parts(raw) {
        let Some((from, to)) = entry.split_once("->") else {
            return Err(format!("invalid {label} entry: {entry}"));
        };
        result.push((
            parse_value(from.trim(), label)?,
            parse_value(to.trim(), label)?,
        ));
    }
    Ok(result)
}

pub(in super::super) fn parse_channel_mappings(raw: &str) -> Result<Vec<ChannelMapEntry>, String> {
    let mut mappings = Vec::new();
    for (from, to) in parse_arrow_entries::<u8>(raw, "channel mapping")? {
        mappings.push(ChannelMapEntry { from, to });
    }
    Ok(mappings)
}

pub(in super::super) fn format_channel_mappings(mappings: &[ChannelMapEntry]) -> String {
    mappings
        .iter()
        .map(|mapping| format!("{}->{}", mapping.from, mapping.to))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(in super::super) fn parse_track_mappings(raw: &str) -> Result<Vec<TrackMapEntry>, String> {
    let mut mappings = Vec::new();
    for (from, to) in parse_arrow_entries::<usize>(raw, "track mapping")? {
        mappings.push(TrackMapEntry { from, to });
    }
    Ok(mappings)
}

pub(in super::super) fn format_track_mappings(mappings: &[TrackMapEntry]) -> String {
    mappings
        .iter()
        .map(|mapping| format!("{}->{}", mapping.from, mapping.to))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(in super::super) fn parse_channel_programs(raw: &str) -> Result<Vec<ChannelProgram>, String> {
    let mut programs = Vec::new();
    for entry in csv_parts(raw) {
        let Some((channel, program)) = entry.split_once(':') else {
            return Err(format!("invalid startup program: {entry}"));
        };
        programs.push(ChannelProgram {
            channel: parse_value(channel.trim(), "startup channel")?,
            program: parse_value(program.trim(), "startup program")?,
        });
    }
    Ok(programs)
}

pub(in super::super) fn format_channel_programs(programs: &[ChannelProgram]) -> String {
    programs
        .iter()
        .map(|program| format!("{}:{}", program.channel, program.program))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(in super::super) fn parse_u8_list(raw: &str, label: &str) -> Result<Vec<u8>, String> {
    csv_parts(raw)
        .map(|entry| parse_value(entry, label))
        .collect()
}

pub(in super::super) fn format_u8_list(values: &[u8]) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

pub(in super::super) fn parse_controller_mappings(
    raw: &str,
) -> Result<Vec<ControllerMapEntry>, String> {
    let mut mappings = Vec::new();
    for (from, to) in parse_arrow_entries::<u8>(raw, "controller mapping")? {
        mappings.push(ControllerMapEntry { from, to });
    }
    Ok(mappings)
}

pub(in super::super) fn format_controller_mappings(mappings: &[ControllerMapEntry]) -> String {
    mappings
        .iter()
        .map(|mapping| format!("{}->{}", mapping.from, mapping.to))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(in super::super) fn parse_controller_scales(
    raw: &str,
) -> Result<Vec<ControllerScaleEntry>, String> {
    let mut entries = Vec::new();
    for entry in csv_parts(raw) {
        let Some((controller, scale)) = entry.split_once(':') else {
            return Err(format!("invalid controller scale: {entry}"));
        };
        entries.push(ControllerScaleEntry {
            controller: parse_value(controller.trim(), "scale controller")?,
            scale: parse_value(scale.trim(), "scale factor")?,
        });
    }
    Ok(entries)
}

pub(in super::super) fn format_controller_scales(entries: &[ControllerScaleEntry]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}:{}", entry.controller, entry.scale))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(in super::super) fn parse_control_values(raw: &str) -> Result<Vec<ControlValue>, String> {
    let mut values = Vec::new();
    for entry in csv_parts(raw) {
        let Some((channel, controller_value)) = entry.split_once(':') else {
            return Err(format!("invalid controller value: {entry}"));
        };
        let Some((controller, value)) = controller_value.split_once('=') else {
            return Err(format!("invalid controller value: {entry}"));
        };
        values.push(ControlValue {
            channel: parse_value(channel.trim(), "inject channel")?,
            controller: parse_value(controller.trim(), "inject controller")?,
            value: parse_value(value.trim(), "inject value")?,
        });
    }
    Ok(values)
}

pub(in super::super) fn format_control_values(values: &[ControlValue]) -> String {
    values
        .iter()
        .map(|value| format!("{}:{}={}", value.channel, value.controller, value.value))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(in super::super) fn parse_velocity_points(raw: &str) -> Result<Vec<VelocityPoint>, String> {
    let mut points = Vec::new();
    for entry in csv_parts(raw) {
        let Some((input, output)) = entry.split_once(':') else {
            return Err(format!("invalid velocity point: {entry}"));
        };
        points.push(VelocityPoint {
            input: parse_value(input.trim(), "velocity input")?,
            output: parse_value(output.trim(), "velocity output")?,
        });
    }
    Ok(points)
}

pub(in super::super) fn format_velocity_points(points: &[VelocityPoint]) -> String {
    points
        .iter()
        .map(|point| format!("{}:{}", point.input, point.output))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(in super::super) fn parse_key_mappings(raw: &str) -> Result<Vec<KeyMapEntry>, String> {
    let mut mappings = Vec::new();
    for (from, to) in parse_arrow_entries::<u8>(raw, "key mapping")? {
        mappings.push(KeyMapEntry { from, to });
    }
    Ok(mappings)
}

pub(in super::super) fn format_key_mappings(mappings: &[KeyMapEntry]) -> String {
    mappings
        .iter()
        .map(|mapping| format!("{}->{}", mapping.from, mapping.to))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(in super::super) fn parse_key_range(raw: &str) -> Result<Option<KeyRange>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let Some((min, max)) = trimmed.split_once('-') else {
        return Err(format!("invalid fold range: {trimmed}"));
    };
    Ok(Some(KeyRange {
        min: parse_value(min.trim(), "fold range min")?,
        max: parse_value(max.trim(), "fold range max")?,
    }))
}

pub(in super::super) fn format_key_range(range: &Option<KeyRange>) -> String {
    range
        .as_ref()
        .map(|range| format!("{}-{}", range.min, range.max))
        .unwrap_or_default()
}

pub(in super::super) fn parse_text_kinds(raw: &str) -> Result<Vec<TextKind>, String> {
    csv_parts(raw)
        .map(|entry| parse_serde_enum(entry, "text kind"))
        .collect()
}

pub(in super::super) fn format_text_kinds(kinds: &[TextKind]) -> String {
    kinds
        .iter()
        .map(format_serde_enum)
        .collect::<Vec<_>>()
        .join(", ")
}

pub(in super::super) fn parse_sysex_messages(raw: &str) -> Result<Vec<Vec<u8>>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut messages = Vec::new();
    for message in trimmed
        .split('|')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let mut bytes = Vec::new();
        for token in message
            .split(|character: char| character.is_ascii_whitespace() || character == ',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
        {
            bytes.push(parse_u8_token(token, "sysex byte")?);
        }
        messages.push(bytes);
    }
    Ok(messages)
}

pub(in super::super) fn format_sysex_messages(messages: &[Vec<u8>]) -> String {
    messages
        .iter()
        .map(|message| {
            message
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

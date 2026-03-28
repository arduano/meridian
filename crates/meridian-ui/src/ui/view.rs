slint::slint! {
    export struct InspectorRow {
        section: string,
        label: string,
        value: string,
    }

    component ActionButton inherits Rectangle {
        in property <string> label;
        in property <bool> active: false;
        callback pressed;

        min-width: 72px;
        min-height: 32px;
        border-radius: 8px;
        border-width: 1px;
        border-color: active ? #7fb1ff : #415777;
        background: touch.pressed ? #2a3d58 : active ? #48658f : #162334;

        Text {
            text: parent.label;
            color: active ? #f4f8ff : #d4e0f4;
            horizontal-alignment: center;
            vertical-alignment: center;
            font-family: "monospace";
            font-size: 12px;
            font-weight: 700;
        }

        touch := TouchArea { clicked => { root.pressed(); } }
    }

    component IconButton inherits Rectangle {
        in property <string> glyph;
        in property <bool> active: false;
        callback pressed;

        min-width: 38px;
        min-height: 38px;
        border-radius: 10px;
        border-width: 1px;
        border-color: active ? #7fb1ff : #415777;
        background: touch.pressed ? #2a3d58 : active ? #48658f : #162334;

        Text {
            text: root.glyph;
            color: active ? #f4f8ff : #d4e0f4;
            horizontal-alignment: center;
            vertical-alignment: center;
            font-family: "monospace";
            font-size: 16px;
            font-weight: 700;
        }

        touch := TouchArea { clicked => { root.pressed(); } }
    }

    component StatTile inherits Rectangle {
        in property <string> label;
        in property <string> value;

        border-radius: 12px;
        border-width: 1px;
        border-color: #2e4763;
        background: #152130;
        min-height: 62px;

        VerticalLayout {
            padding: 10px;
            spacing: 3px;

            Text {
                text: root.label;
                color: #86a2c7;
                font-family: "monospace";
                font-size: 10px;
            }

            Text {
                text: root.value;
                color: #edf5ff;
                font-family: "monospace";
                font-size: 14px;
                font-weight: 700;
                overflow: elide;
            }
        }
    }

    component InspectorLine inherits Rectangle {
        in property <string> section;
        in property <string> label;
        in property <string> value;

        min-height: 46px;
        border-radius: 10px;
        border-width: 1px;
        border-color: #294058;
        background: #111c29;

        HorizontalLayout {
            padding: 10px;
            spacing: 8px;

            Rectangle {
                width: 66px;
                border-radius: 999px;
                background: #1f3145;
                border-width: 1px;
                border-color: #35516f;

                Text {
                    text: root.section;
                    color: #9fc0ea;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                    font-family: "monospace";
                    font-size: 10px;
                }
            }

            VerticalLayout {
                spacing: 2px;
                Text {
                    text: root.label;
                    color: #dbe8fa;
                    font-size: 12px;
                    font-weight: 600;
                }
                Text {
                    text: root.value;
                    color: #86a2c7;
                    font-family: "monospace";
                    font-size: 11px;
                    overflow: elide;
                }
            }
        }
    }

    component TimeScrubber inherits Rectangle {
        in property <float> value: 0;
        in property <float> maximum: 1;
        callback scrubbed(float);

        private property <float> clamped-value: value < 0 ? 0 : value > maximum ? maximum : value;
        private property <float> fill-ratio: maximum <= 0 ? 0 : clamped-value / maximum;

        min-height: 36px;
        border-radius: 12px;
        border-width: 1px;
        border-color: #324f70;
        background: #101927;

        Rectangle {
            x: 8px;
            y: parent.height / 2 - 4px;
            width: parent.width - 16px;
            height: 8px;
            border-radius: 999px;
            background: #1d2c40;
        }

        Rectangle {
            x: 8px;
            y: parent.height / 2 - 4px;
            width: (parent.width - 16px) * root.fill-ratio;
            height: 8px;
            border-radius: 999px;
            background: #4dc1ff;
        }

        Rectangle {
            x: 8px + (parent.width - 16px) * root.fill-ratio - 7px;
            y: parent.height / 2 - 9px;
            width: 14px;
            height: 18px;
            border-radius: 999px;
            background: #edf6ff;
            border-width: 1px;
            border-color: #173a56;
        }

        touch := TouchArea {
            moved => {
                if (self.pressed) {
                    root.scrubbed(maximum * max(0, min(1, (self.mouse-x - 8px) / max(1px, parent.width - 16px))));
                }
            }
            clicked => {
                root.scrubbed(maximum * max(0, min(1, (self.mouse-x - 8px) / max(1px, parent.width - 16px))));
            }
        }
    }

    export component App inherits Window {
        in-out property <image> viewport-image;
        in-out property <string> midi-path-text: "No MIDI loaded";
        in-out property <string> status-text: "Waiting for MIDI";
        in-out property <string> time-text: "0.000 s";
        in-out property <string> length-text: "0.000 s";
        in-out property <string> note-count-text: "0";
        in-out property <string> visible-note-count-text: "0";
        in-out property <string> active-keys-text: "0";
        in-out property <string> view-range-text: "8.0 s";
        in-out property <string> play-label: "Play";
        in-out property <string> fps-text: "--";
        in-out property <string> scene-summary-text: "2D / PFA notes / PFA keyboard";
        in-out property <string> viewport-text: "1280 x 720";
        in-out property <string> current-renderer-text: "pfa";
        in-out property <float> current-time-seconds: 0;
        in-out property <float> midi-length-seconds: 1;
        in-out property <[InspectorRow]> inspector-items;

        out property <float> viewport-px-width: viewport-box.width / 1px;
        out property <float> viewport-px-height: viewport-box.height / 1px;

        callback step-time(float);
        callback zoom(float);
        callback toggle-play();
        callback seek-time(float);
        callback select-renderer(string);

        title: "Meridian";
        preferred-width: 1540px;
        preferred-height: 940px;
        background: rgb(8, 14, 24);

        VerticalLayout {
            padding: 14px;
            spacing: 10px;

            Rectangle {
                border-radius: 16px;
                border-width: 1px;
                border-color: #263b56;
                background: #111b29;
                min-height: 76px;
                max-height: 76px;

                HorizontalLayout {
                    padding: 12px;
                    spacing: 10px;

                    VerticalLayout {
                        width: 170px;
                        spacing: 2px;
                        Text {
                            text: "Meridian";
                            color: #f0f6ff;
                            font-size: 18px;
                            font-weight: 700;
                        }
                        Text {
                            text: root.midi-path-text;
                            color: #89aad1;
                            font-size: 11px;
                            overflow: elide;
                        }
                    }

                    IconButton { glyph: root.play-label == "Pause" ? "||" : ">"; active: root.play-label == "Pause"; pressed => { root.toggle-play(); } }
                    IconButton { glyph: "-"; pressed => { root.zoom(-1.0); } }

                    TimeScrubber {
                        horizontal-stretch: 1;
                        min-width: 320px;
                        value: root.current-time-seconds;
                        maximum: root.midi-length-seconds;
                        scrubbed(value) => { root.seek-time(value); }
                    }

                    IconButton { glyph: "+"; pressed => { root.zoom(1.0); } }

                    VerticalLayout {
                        width: 58px;
                        spacing: 2px;
                        Text {
                            text: "VIEW";
                            color: #86a2c7;
                            font-family: "monospace";
                            font-size: 9px;
                            horizontal-alignment: right;
                        }
                        Text {
                            text: root.view-range-text;
                            color: #edf5ff;
                            font-family: "monospace";
                            font-size: 14px;
                            font-weight: 700;
                            horizontal-alignment: right;
                        }
                    }
                }
            }

            HorizontalLayout {
                spacing: 10px;

                Rectangle {
                    border-radius: 18px;
                    border-width: 1px;
                    border-color: #28415d;
                    background: #0e1725;
                    horizontal-stretch: 1;
                    vertical-stretch: 1;

                    VerticalLayout {
                        padding: 10px;
                        spacing: 8px;

                        Rectangle {
                            border-radius: 12px;
                            border-width: 1px;
                            border-color: #314c6d;
                            background: #132032;
                            min-height: 50px;

                            HorizontalLayout {
                                padding: 10px;
                                spacing: 8px;
                                Text {
                                    text: "Viewport";
                                    color: #eff5ff;
                                    font-size: 16px;
                                    font-weight: 700;
                                }
                                Rectangle { horizontal-stretch: 1; background: #00000000; }
                                StatTile { label: "FPS"; value: root.fps-text; }
                                StatTile { label: "SIZE"; value: root.viewport-text; }
                            }
                        }

                        viewport-box := Rectangle {
                            border-radius: 16px;
                            border-width: 1px;
                            border-color: #35516f;
                            background: #08111c;
                            horizontal-stretch: 1;
                            vertical-stretch: 1;

                            Rectangle {
                                x: 16px;
                                y: 16px;
                                width: parent.width - 32px;
                                height: parent.height - 32px;
                                border-radius: 12px;
                                border-width: 1px;
                                border-color: #456789;
                                background: #07111d;

                                Image {
                                    source: root.viewport-image;
                                    width: parent.width;
                                    height: parent.height;
                                    image-fit: fill;
                                }
                            }
                        }
                    }
                }

                Rectangle {
                    width: 360px;
                    border-radius: 18px;
                    border-width: 1px;
                    border-color: #263b56;
                    background: #101926;

                    VerticalLayout {
                        padding: 10px;
                        spacing: 8px;

                        Text {
                            text: "Live Stats";
                            color: #f0f6ff;
                            font-family: "monospace";
                            font-size: 15px;
                            font-weight: 700;
                        }

                        GridLayout {
                            spacing: 8px;
                            StatTile { label: "MIDI"; value: root.midi-path-text; }
                            StatTile { label: "VISIBLE"; value: root.visible-note-count-text; }
                            StatTile { label: "ACTIVE KEYS"; value: root.active-keys-text; }
                            StatTile { label: "TOTAL NOTES"; value: root.note-count-text; }
                        }

                        Rectangle {
                            border-radius: 14px;
                            border-width: 1px;
                            border-color: #2e4763;
                            background: #131f2d;
                            min-height: 76px;

                            VerticalLayout {
                                padding: 10px;
                                spacing: 8px;

                                Text {
                                    text: "Projector";
                                    color: #8fb3df;
                                    font-family: "monospace";
                                    font-size: 11px;
                                }

                                HorizontalLayout {
                                    spacing: 8px;
                                    ActionButton { label: "PFA"; active: root.current-renderer-text == "pfa"; pressed => { root.select-renderer("pfa"); } }
                                    ActionButton { label: "Flat"; active: root.current-renderer-text == "flat"; pressed => { root.select-renderer("flat"); } }
                                }
                            }
                        }

                        Rectangle {
                            border-radius: 14px;
                            border-width: 1px;
                            border-color: #2e4763;
                            background: #131f2d;
                            min-height: 86px;

                            VerticalLayout {
                                padding: 10px;
                                spacing: 4px;
                                Text {
                                    text: "Status";
                                    color: #8fb3df;
                                    font-family: "monospace";
                                    font-size: 11px;
                                }
                                Text {
                                    text: root.status-text;
                                    color: #deebff;
                                    font-size: 12px;
                                    wrap: word-wrap;
                                }
                            }
                        }

                        Rectangle {
                            border-radius: 14px;
                            border-width: 1px;
                            border-color: #2e4763;
                            background: #0f1824;
                            vertical-stretch: 1;

                            VerticalLayout {
                                padding: 10px;
                                spacing: 8px;

                                Text {
                                    text: "Advanced Inspector";
                                    color: #f0f6ff;
                                    font-family: "monospace";
                                    font-size: 13px;
                                    font-weight: 700;
                                }

                                Text {
                                    text: "First pass: generated property list from the active scene config. Next pass can turn each row into a typed editor.";
                                    color: #88a5ca;
                                    font-size: 11px;
                                    wrap: word-wrap;
                                }

                                Rectangle {
                                    vertical-stretch: 1;
                                    background: #00000000;
                                    clip: true;

                                    VerticalLayout {
                                        width: parent.width;
                                        spacing: 6px;
                                        for row in root.inspector-items : InspectorLine {
                                            section: row.section;
                                            label: row.label;
                                            value: row.value;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

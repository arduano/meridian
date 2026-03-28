slint::slint! {
    component ActionButton inherits Rectangle {
        in property <string> label;
        in property <bool> active: false;
        callback pressed;

        min-width: 70px;
        min-height: 30px;
        border-radius: 7px;
        border-width: 1px;
        border-color: active ? #77a7ff : #44576d;
        background: touch.pressed ? #2a3b52 : active ? #405980 : #1a2331;

        Text {
            text: parent.label;
            color: active ? #f4f7ff : #d2def2;
            horizontal-alignment: center;
            vertical-alignment: center;
            font-family: "monospace";
            font-size: 12px;
            font-weight: 700;
        }

        touch := TouchArea { clicked => { root.pressed(); } }
    }

    component StatLine inherits Rectangle {
        in property <string> label;
        in property <string> value;

        min-height: 48px;
        border-radius: 10px;
        border-width: 1px;
        border-color: #32455d;
        background: #172334;

        VerticalLayout {
            padding: 8px;
            spacing: 2px;

            Text {
                text: label;
                color: #89a0bf;
                font-family: "monospace";
                font-size: 11px;
            }

            Text {
                text: value;
                color: #edf4ff;
                font-family: "monospace";
                font-size: 13px;
                font-weight: 700;
                overflow: elide;
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

        out property <float> viewport-px-width: viewport-box.width / 1px;
        out property <float> viewport-px-height: viewport-box.height / 1px;

        callback step-time(float);
        callback zoom(float);
        callback toggle-play();

        title: "Meridian";
        preferred-width: 1480px;
        preferred-height: 920px;
        background: rgb(11, 18, 32);

        HorizontalLayout {
            padding: 14px;
            spacing: 12px;

            Rectangle {
                width: 296px;
                border-radius: 18px;
                border-width: 1px;
                border-color: #26354d;
                background: #121b29;

                VerticalLayout {
                    padding: 12px;
                    spacing: 10px;

                    Text {
                        text: "MERIDIAN // BLUE CRT";
                        color: rgb(238, 245, 255);
                        font-family: "monospace";
                        font-size: 18px;
                        font-weight: 700;
                    }

                    Text {
                        text: "Retro workstation shell with a native note viewport embedded into the main panel.";
                        color: #8ca3c5;
                        font-size: 12px;
                        wrap: word-wrap;
                    }

                    Rectangle { height: 1px; background: #223147; }

                    StatLine { label: "MIDI"; value: root.midi-path-text; }
                    StatLine { label: "TIME"; value: root.time-text; }
                    StatLine { label: "LENGTH"; value: root.length-text; }
                    StatLine { label: "FPS"; value: root.fps-text; }
                    StatLine { label: "VISIBLE"; value: root.visible-note-count-text; }
                    StatLine { label: "ACTIVE KEYS"; value: root.active-keys-text; }
                    StatLine { label: "TOTAL NOTES"; value: root.note-count-text; }
                    StatLine { label: "VIEW RANGE"; value: root.view-range-text; }

                    Rectangle {
                        border-radius: 14px;
                        border-width: 1px;
                        border-color: #314760;
                        background: #172130;

                        VerticalLayout {
                            padding: 10px;
                            spacing: 8px;

                            Text {
                                text: "TRANSPORT";
                                color: #87a7d6;
                                font-family: "monospace";
                                font-size: 12px;
                            }

                            HorizontalLayout {
                                spacing: 6px;
                                ActionButton { label: "-5s"; pressed => { root.step-time(-5.0); } }
                                ActionButton { label: "-1s"; pressed => { root.step-time(-1.0); } }
                                ActionButton { label: "+1s"; pressed => { root.step-time(1.0); } }
                                ActionButton { label: "+5s"; pressed => { root.step-time(5.0); } }
                            }

                            HorizontalLayout {
                                spacing: 6px;
                                ActionButton { label: root.play-label; active: root.play-label == "Pause"; pressed => { root.toggle-play(); } }
                                ActionButton { label: "Zoom-"; pressed => { root.zoom(-1.0); } }
                                ActionButton { label: "Zoom+"; pressed => { root.zoom(1.0); } }
                            }
                        }
                    }

                    Rectangle {
                        border-radius: 14px;
                        border-width: 1px;
                        border-color: #2d4360;
                        background: #15202f;

                        VerticalLayout {
                            padding: 10px;
                            spacing: 4px;

                            Text {
                                text: "STATUS";
                                color: #87a7d6;
                                font-family: "monospace";
                                font-size: 12px;
                            }

                            Text {
                                text: root.status-text;
                                color: rgb(217, 232, 255);
                                font-family: "monospace";
                                font-size: 12px;
                                wrap: word-wrap;
                            }
                        }
                    }

                    Rectangle {
                        vertical-stretch: 1;
                        border-radius: 14px;
                        background: #101926;
                        border-width: 1px;
                        border-color: #203046;
                    }
                }
            }

            Rectangle {
                border-radius: 18px;
                border-width: 1px;
                border-color: #26354d;
                background: #101826;
                horizontal-stretch: 1;
                vertical-stretch: 1;

                VerticalLayout {
                    padding: 10px;
                    spacing: 8px;

                    Rectangle {
                        border-radius: 12px;
                        border-width: 1px;
                        border-color: #31455f;
                        background: #162131;

                        HorizontalLayout {
                            padding: 10px;
                            spacing: 8px;

                            Text {
                                text: "Viewport";
                                color: #f1f6ff;
                                font-size: 16px;
                                font-weight: 700;
                            }

                            Rectangle { horizontal-stretch: 1; background: #00000000; }

                            Rectangle {
                                border-radius: 999px;
                                border-width: 1px;
                                border-color: #496584;
                                background: #23344a;
                                min-width: 112px;
                                min-height: 26px;

                                Text {
                                    text: "CRT / NOTE FIELD";
                                    color: #c7dbfb;
                                    horizontal-alignment: center;
                                    vertical-alignment: center;
                                    font-family: "monospace";
                                    font-size: 11px;
                                }
                            }
                        }
                    }

                    viewport-box := Rectangle {
                        border-radius: 16px;
                        border-width: 1px;
                        border-color: rgb(52, 81, 110);
                        background: rgb(13, 22, 36);
                        horizontal-stretch: 1;
                        vertical-stretch: 1;

                        Rectangle {
                            x: 18px;
                            y: 18px;
                            width: parent.width - 36px;
                            height: parent.height - 36px;
                            border-radius: 12px;
                            border-width: 1px;
                            border-color: #456789;
                            background: rgb(7, 17, 29);

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
        }
    }
}

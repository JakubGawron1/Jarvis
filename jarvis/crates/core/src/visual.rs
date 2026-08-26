use jarvis_protocol::{
    Body3d, Diagram, Orbit, Scene3d, Slide, VideoClip, VisualKind, VisualSpec,
};

pub fn wants_visual(text: &str) -> bool {
    let t = text.to_lowercase();
    const KEYS: &[&str] = &[
        "pokaż", "pokaz", "pokaż mi", "show ", "show me", "visualize",
        "narysuj", "wykres", "chart", "prezentacj", "presentation", "slides",
        "animacj", "hologram", "model ", "modelu", "3d", "film", "video",
        "diagram", "schemat", "mapa", "atom", "dna", "układ słonecz",
        "solar", "glob", "ziemia", "earth",
    ];
    KEYS.iter().any(|k| t.contains(k))
}

/// Build a visual for *any* topic. Special cases (atom, solar system, …) get
/// richer geometry; everything else becomes an amber neural hologram labeled
/// with the topic — same idea as a lab HUD, not a single hardcoded demo.
pub fn visual_from_prompt(text: &str) -> VisualSpec {
    let t = text.to_lowercase();
    let title = topic_title(text);

    if t.contains("prezentacj") || t.contains("presentation") || t.contains("slides") {
        return slides_about(&title);
    }
    if t.contains("wykres") || t.contains("chart") || t.contains("diagram") || t.contains("schemat")
    {
        return diagram_about(&title);
    }

    let mut spec = if looks_like_atom(&t) {
        atom_scene(&t, &title)
    } else if t.contains("układ słonecz") || t.contains("solar") || t.contains("planet") {
        solar_scene(&title)
    } else if t.contains("dna") {
        dna_scene(&title)
    } else if t.contains("ziemia") || t.contains("earth") || t.contains("glob") {
        globe_scene(&title)
    } else {
        neural_hologram(&title)
    };

    if t.contains("film") || t.contains("video") || t.contains("animacj") {
        spec.kind = VisualKind::Video;
        spec.video = Some(VideoClip {
            duration_sec: 10.0,
            caption: Some(title.clone()),
        });
    }
    spec
}

fn looks_like_atom(t: &str) -> bool {
    t.contains("atom")
        || t.contains("bohr")
        || t.contains("elektron")
        || t.contains("hydrogen")
        || t.contains("wodór")
        || t.contains("helium")
        || t.contains("węgiel")
        || t.contains("carbon")
}

fn topic_title(text: &str) -> String {
    let stripped = text
        .replace("pokaż mi", "")
        .replace("pokaż", "")
        .replace("pokaz", "")
        .replace("show me", "")
        .replace("show", "")
        .replace("narysuj", "")
        .replace("visualize", "")
        .replace("model", "")
        .replace("3d", "")
        .trim()
        .trim_matches(|c: char| c == ':' || c == ',' || c == '.')
        .to_string();
    if stripped.is_empty() {
        "Hologram".into()
    } else {
        let mut c = stripped.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => stripped,
        }
    }
}

fn neural_hologram(title: &str) -> VisualSpec {
    let n = 28usize;
    let mut bodies = Vec::new();
    let mut links = Vec::new();
    for i in 0..n {
        bodies.push(Body3d {
            id: format!("n{i}"),
            shape: "sphere".into(),
            radius: 0.08,
            color: "#ff8c3a".into(),
            glow: true,
            orbit: Some(Orbit {
                radius: 2.2 + (i as f32 % 5.0) * 0.25,
                speed: 0.15 + (i as f32) * 0.02,
                tilt: (i as f32) * 0.22,
            }),
            label: None,
        });
        links.push((i, (i * 7 + 3) % n));
        links.push((i, (i * 5 + 11) % n));
    }
    bodies.push(Body3d {
        id: "core".into(),
        shape: "sphere".into(),
        radius: 0.35,
        color: "#ffb347".into(),
        glow: true,
        orbit: None,
        label: Some(title.chars().take(24).collect()),
    });
    VisualSpec {
        kind: VisualKind::Scene3d,
        title: title.into(),
        subtitle: Some("holographic construct".into()),
        scene3d: Some(Scene3d {
            camera_z: 9.0,
            bodies,
            links,
            particles: 900,
            neural: true,
        }),
        slides: None,
        diagram: None,
        video: None,
    }
}

fn atom_scene(t: &str, title: &str) -> VisualSpec {
    let electrons = if t.contains("helium") || t.contains("helu") {
        2
    } else if t.contains("carbon") || t.contains("węgiel") {
        6
    } else if t.contains("oxygen") || t.contains("tlen") {
        8
    } else {
        1
    };
    let mut bodies = vec![Body3d {
        id: "nucleus".into(),
        shape: "sphere".into(),
        radius: 0.42,
        color: "#ff6a1a".into(),
        glow: true,
        orbit: None,
        label: Some("nucleus".into()),
    }];
    for i in 0..electrons {
        bodies.push(Body3d {
            id: format!("e{i}"),
            shape: "sphere".into(),
            radius: 0.09,
            color: "#4ee3ff".into(),
            glow: true,
            orbit: Some(Orbit {
                radius: 1.6 + (i / 2) as f32 * 0.7,
                speed: 0.8 + i as f32 * 0.15,
                tilt: i as f32 * 0.55,
            }),
            label: if i == 0 { Some("e⁻".into()) } else { None },
        });
    }
    VisualSpec {
        kind: VisualKind::Scene3d,
        title: title.into(),
        subtitle: Some(format!("Bohr-style · {electrons} e⁻")),
        scene3d: Some(Scene3d {
            camera_z: 7.0,
            bodies,
            links: vec![],
            particles: 200,
            neural: false,
        }),
        slides: None,
        diagram: None,
        video: None,
    }
}

fn solar_scene(title: &str) -> VisualSpec {
    let planets = [
        ("Sun", 0.7, "#ffb347", 0.0, 0.0),
        ("Mercury", 0.08, "#c4a574", 1.3, 1.6),
        ("Venus", 0.12, "#e8c07a", 1.8, 1.2),
        ("Earth", 0.13, "#4ea3ff", 2.4, 1.0),
        ("Mars", 0.1, "#ff6a3a", 3.0, 0.8),
        ("Jupiter", 0.28, "#d4a574", 4.0, 0.45),
    ];
    let bodies = planets
        .iter()
        .enumerate()
        .map(|(i, (name, r, color, orbit, speed))| Body3d {
            id: name.to_lowercase(),
            shape: "sphere".into(),
            radius: *r,
            color: (*color).into(),
            glow: i == 0,
            orbit: if i == 0 {
                None
            } else {
                Some(Orbit {
                    radius: *orbit,
                    speed: *speed,
                    tilt: 0.05,
                })
            },
            label: Some((*name).into()),
        })
        .collect();
    VisualSpec {
        kind: VisualKind::Scene3d,
        title: title.into(),
        subtitle: Some("inner system".into()),
        scene3d: Some(Scene3d {
            camera_z: 10.0,
            bodies,
            links: vec![],
            particles: 120,
            neural: false,
        }),
        slides: None,
        diagram: None,
        video: None,
    }
}

fn dna_scene(title: &str) -> VisualSpec {
    let mut bodies = Vec::new();
    for i in 0..24 {
        let y = (i as f32) * 0.22 - 2.5;
        bodies.push(Body3d {
            id: format!("a{i}"),
            shape: "sphere".into(),
            radius: 0.1,
            color: if i % 2 == 0 { "#ff8c3a".into() } else { "#4ee3ff".into() },
            glow: true,
            orbit: Some(Orbit {
                radius: 1.1,
                speed: 0.4,
                tilt: y,
            }),
            label: None,
        });
    }
    VisualSpec {
        kind: VisualKind::Scene3d,
        title: title.into(),
        subtitle: Some("double helix".into()),
        scene3d: Some(Scene3d {
            camera_z: 8.0,
            bodies,
            links: vec![],
            particles: 80,
            neural: false,
        }),
        slides: None,
        diagram: None,
        video: None,
    }
}

fn globe_scene(title: &str) -> VisualSpec {
    VisualSpec {
        kind: VisualKind::Scene3d,
        title: title.into(),
        subtitle: Some("globe".into()),
        scene3d: Some(Scene3d {
            camera_z: 5.0,
            bodies: vec![Body3d {
                id: "earth".into(),
                shape: "sphere".into(),
                radius: 1.4,
                color: "#3d8bff".into(),
                glow: true,
                orbit: None,
                label: Some("Earth".into()),
            }],
            links: vec![],
            particles: 400,
            neural: false,
        }),
        slides: None,
        diagram: None,
        video: None,
    }
}

fn slides_about(title: &str) -> VisualSpec {
    VisualSpec {
        kind: VisualKind::Slides,
        title: title.into(),
        subtitle: None,
        scene3d: None,
        slides: Some(vec![
            Slide {
                title: title.into(),
                bullets: vec![
                    "Construct generated by Jarvis".into(),
                    "Ask for more detail or a 3D model".into(),
                ],
            },
            Slide {
                title: "Overview".into(),
                bullets: vec![
                    format!("Subject: {title}"),
                    "Holographic briefing".into(),
                    "Continue in chat for depth".into(),
                ],
            },
            Slide {
                title: "Next".into(),
                bullets: vec!["Say “pokaż model 3D” for a hologram.".into()],
            },
        ]),
        diagram: None,
        video: None,
    }
}

fn diagram_about(title: &str) -> VisualSpec {
    VisualSpec {
        kind: VisualKind::Diagram,
        title: title.into(),
        subtitle: None,
        scene3d: None,
        slides: None,
        diagram: Some(Diagram {
            nodes: vec![
                title.into(),
                "Input".into(),
                "Process".into(),
                "Output".into(),
            ],
            edges: vec![(1, 0), (0, 2), (2, 3)],
        }),
        video: None,
    }
}

pub fn parse_visual_tag(reply: &str) -> Option<VisualSpec> {
    let start = reply.find("[[visual:")?;
    let rest = &reply[start + 9..];
    let end = rest.find("]]")?;
    serde_json::from_str(rest[..end].trim()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_show_atom() {
        assert!(wants_visual("pokaż model atomu"));
        assert!(wants_visual("show me DNA"));
        assert!(wants_visual("zrób prezentację o fotosyntezie"));
        assert!(!wants_visual("jaki mam kalendarz"));
    }

    #[test]
    fn atom_is_not_generic_neural() {
        let v = visual_from_prompt("pokaż model atomu helu");
        assert_eq!(v.kind, VisualKind::Scene3d);
        let scene = v.scene3d.expect("scene");
        assert!(scene.bodies.iter().any(|b| b.id == "nucleus"));
        assert_eq!(scene.bodies.iter().filter(|b| b.id.starts_with('e')).count(), 2);
    }

    #[test]
    fn unknown_topic_gets_neural_hologram() {
        let v = visual_from_prompt("pokaż mi fotosyntezę");
        assert!(v.scene3d.unwrap().neural);
        assert!(v.title.to_lowercase().contains("fotosyntez"));
    }

    #[test]
    fn parses_visual_tag() {
        let spec = parse_visual_tag(r#"ok [[visual:{"kind":"slides","title":"X"}]]"#).unwrap();
        assert_eq!(spec.kind, VisualKind::Slides);
        assert_eq!(spec.title, "X");
    }
}

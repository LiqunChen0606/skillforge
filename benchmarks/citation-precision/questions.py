"""
Ground-truth Q&A pairs for citation precision benchmark.

Each document has a set of questions with:
- question: The question to ask
- answer_keywords: Key phrases that must appear in a correct answer
- source_section_ids: Section IDs where the answer can be found
- difficulty: easy (single section) / medium (cross-section) / hard (requires inference)
"""

GROUND_TRUTH = {
    "examples/documents/wiki_article.aif": {
        "name": "Photosynthesis",
        "questions": [
            {
                "question": "What is the overall chemical equation for photosynthesis?",
                "answer_keywords": ["6CO₂", "6H₂O", "C₆H₁₂O₆", "6O₂", "light energy"],
                "source_section_ids": ["equation"],
                "difficulty": "easy",
            },
            {
                "question": "What experimental evidence proved that oxygen comes from water during photosynthesis?",
                "answer_keywords": ["isotope", "H₂¹⁸O", "water"],
                "source_section_ids": ["light-reactions"],
                "difficulty": "easy",
            },
            {
                "question": "What are the two main stages of photosynthesis and where do they occur?",
                "answer_keywords": ["light-dependent", "Calvin cycle", "thylakoid", "stroma"],
                "source_section_ids": ["stages", "light-reactions", "calvin-cycle"],
                "difficulty": "medium",
            },
            {
                "question": "What is the role of RuBisCO in photosynthesis?",
                "answer_keywords": ["carbon", "fixation", "CO₂", "enzyme"],
                "source_section_ids": ["calvin-cycle"],
                "difficulty": "medium",
            },
            {
                "question": "How does the light-dependent stage connect to the Calvin cycle?",
                "answer_keywords": ["ATP", "NADPH"],
                "source_section_ids": ["light-reactions", "calvin-cycle"],
                "difficulty": "hard",
            },
        ],
    },
    "examples/documents/simple.aif": {
        "name": "Getting Started with AIF",
        "questions": [
            {
                "question": "What is AIF and what makes it different from Markdown?",
                "answer_keywords": ["semantic", "structured blocks", "claims", "evidence"],
                "source_section_ids": ["what-is-aif"],
                "difficulty": "easy",
            },
            {
                "question": "What output formats does AIF support?",
                "answer_keywords": ["HTML", "Markdown", "LML", "JSON"],
                "source_section_ids": ["features"],
                "difficulty": "easy",
            },
            {
                "question": "How do you import a Markdown file into AIF?",
                "answer_keywords": ["aif import"],
                "source_section_ids": ["example"],
                "difficulty": "easy",
            },
        ],
    },
    "examples/rich-content/climate_data.aif": {
        "name": "Global Temperature Anomalies",
        "questions": [
            {
                "question": "What is a temperature anomaly and what baseline period is used?",
                "answer_keywords": ["departure", "1951-1980", "baseline"],
                "source_section_ids": ["overview"],
                "difficulty": "easy",
            },
            {
                "question": "Which year in 2020-2024 had the highest annual temperature anomaly?",
                "answer_keywords": ["2023", "1.17"],
                "source_section_ids": ["data"],
                "difficulty": "easy",
            },
            {
                "question": "What does the Arctic warming pattern tell us about climate change?",
                "answer_keywords": ["amplif", "polar"],
                "source_section_ids": ["visualization", "analysis"],
                "difficulty": "hard",
            },
            {
                "question": "What evidence supports the claim that 2020-2024 is the warmest five-year period?",
                "answer_keywords": ["temperature", "anomaly", "record"],
                "source_section_ids": ["data", "analysis"],
                "difficulty": "medium",
            },
        ],
    },
}

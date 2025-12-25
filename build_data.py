import json
import random

# --- CONFIGURATION ---
TARGET_COUNT = 20000
OUTPUT_FILE = "shell_instruct_dataset_v2.jsonl"

# --- DATA POOLS ---
FILENAMES = [
    "app",
    "main",
    "server",
    "config",
    "docker-compose",
    "Dockerfile",
    "package",
    "requirements",
    "index",
    "style",
    "utils",
    "test",
    "data",
    "backup",
    "image",
    "logo",
    "user_data",
    "notes",
    "todo",
    "license",
    "Makefile",
    "prod",
    "dev",
]

EXTENSIONS = [
    ".py",
    ".js",
    ".json",
    ".yml",
    ".txt",
    ".md",
    ".log",
    ".sh",
    ".conf",
    ".env",
    ".html",
    ".css",
    ".png",
    ".jpg",
    ".zip",
    ".tar.gz",
    ".sql",
]

PATHS = [
    "./",
    "/var/log",
    "/etc/nginx",
    "~/Documents",
    "/tmp",
    "/opt/app",
    "C:\\Windows\\System32",
    "D:\\Backups",
    "/home/user/projects",
]

SERVICES = [
    "nginx",
    "apache2",
    "docker",
    "postgresql",
    "mysql",
    "ssh",
    "cron",
    "firewalld",
]

SEARCH_TERMS = [
    "error",
    "warning",
    "fail",
    "success",
    "404",
    "exception",
    "TODO",
    "FIXME",
]

# --- NATURAL LANGUAGE VARIATIONS ---
# Maps a standard intent keyword to various natural language ways to say it
NL_MAPPING = {
    "list": [
        "list",
        "show",
        "display",
        "ls",
        "get",
        "what is in",
        "show me contents of",
    ],
    "remove": ["remove", "delete", "erase", "nuke", "wipe", "clear", "get rid of"],
    "create": ["create", "make", "generate", "new", "setup"],
    "move": ["move", "rename", "transfer", "relocate"],
    "copy": ["copy", "duplicate", "clone", "backup"],
    "find": ["find", "search for", "look for", "grep", "locate", "where is"],
    "check": ["check", "monitor", "status of", "how is"],
}


def get_natural_prompt(base_prompt):
    """Replaces standard keywords with random conversational synonyms."""
    words = base_prompt.split()
    new_words = []
    for word in words:
        lower_word = word.lower()
        if lower_word in NL_MAPPING:
            new_words.append(random.choice(NL_MAPPING[lower_word]))
        else:
            new_words.append(word)
    return " ".join(new_words)


# --- TEMPLATES ---

# Simple one-liners
SIMPLE_TEMPLATES = [
    {
        "intent": "list",
        "bash": "ls {flags} {path}",
        "ps": "Get-ChildItem {flags} -Path '{path}'",
        "variations": [
            ("", "", "files in {path}"),
            ("-la", "-Force", "all files including hidden ones in {path}"),
            ("-lh", "", "files with sizes in {path}"),
            ("-R", "-Recurse", "all files recursively in {path}"),
            ("-t", "", "files sorted by modification time in {path}"),
        ],
    },
    {
        "intent": "remove",
        "bash": "rm {flags} {target}",
        "ps": "Remove-Item {flags} -Path '{target}'",
        "variations": [
            ("-rf", "-Recurse -Force", "the folder {target} and everything inside it"),
            ("-f", "-Force", "the file {target} forcefully"),
            ("", "", "the file {target}"),
        ],
        "dangerous": True,
    },
    {
        "intent": "create",
        "bash": "mkdir -p {target}",
        "ps": "New-Item -ItemType Directory -Force -Path '{target}'",
        "variations": [
            ("", "", "a directory named {target}"),
            ("", "", "the folder {target}"),
        ],
    },
    {
        "intent": "find",
        "bash": "grep {flags} '{term}' {target}",
        "ps": "Select-String {flags} -Pattern '{term}' -Path '{target}'",
        "variations": [
            ("-r", "", "the text '{term}' inside {target} folder"),
            ("-i", "", "the string '{term}' in {target} ignoring case"),
            ("-v", "-NotMatch", "lines in {target} that do NOT contain '{term}'"),
            ("-c", "", "count how many times '{term}' appears in {target}"),
        ],
    },
]

# Complex commands (Pipes, Redirection, Chaining)
COMPLEX_TEMPLATES = [
    {
        "type": "pipe_grep",
        "prompt": "find active processes matching '{term}'",
        "bash": "ps aux | grep '{term}'",
        "ps": "Get-Process | Where-Object {{ $_.ProcessName -match '{term}' }}",
        "dangerous": False,
    },
    {
        "type": "redirect_log",
        "prompt": "save the list of running processes to a file named {target}",
        "bash": "ps aux > {target}",
        "ps": "Get-Process | Out-File -FilePath '{target}'",
        "dangerous": False,
    },
    {
        "type": "chain_create_cd",
        "prompt": "create a folder named {target} and go into it",
        "bash": "mkdir {target} && cd {target}",
        "ps": "New-Item -ItemType Directory -Path '{target}'; Set-Location -Path '{target}'",
        "dangerous": False,
    },
    {
        "type": "pipe_count",
        "prompt": "count the number of files in the current directory",
        "bash": "ls -1 | wc -l",
        "ps": "(Get-ChildItem).Count",
        "dangerous": False,
    },
    {
        "type": "dangerous_wipe",
        "prompt": "delete all files in {path} without confirmation",
        "bash": "rm -rf {path}/*",
        "ps": "Remove-Item -Path '{path}\\*' -Recurse -Force",
        "dangerous": True,
    },
    {
        "type": "net_check",
        "prompt": "check if {service} is listening on port {port}",
        "bash": "netstat -tuln | grep {port}",
        "ps": "Get-NetTCPConnection -LocalPort {port}",
        "dangerous": False,
    },
]


def generate_entry():
    """Generates a single random entry."""
    is_complex = random.random() < 0.3  # 30% chance of complex command

    target = f"{random.choice(FILENAMES)}{random.choice(EXTENSIONS)}"
    path = random.choice(PATHS)
    term = random.choice(SEARCH_TERMS)
    service = random.choice(SERVICES)
    port = str(random.randint(1000, 9000))

    if is_complex:
        template = random.choice(COMPLEX_TEMPLATES)
        prompt = template["prompt"].format(
            target=target, path=path, term=term, service=service, port=port
        )
        prompt = get_natural_prompt(prompt)  # Naturalize

        bash_cmd = template["bash"].format(
            target=target, path=path, term=term, service=service, port=port
        )
        ps_cmd = template["ps"].format(
            target=target, path=path, term=term, service=service, port=port
        )

        # Return both variants to balance dataset
        return [
            {
                "prompt": prompt,
                "command": bash_cmd,
                "dangerous": template["dangerous"],
                "shell": "bash",
            },
            {
                "prompt": prompt,
                "command": ps_cmd,
                "dangerous": template["dangerous"],
                "shell": "powershell",
            },
        ]

    else:
        template = random.choice(SIMPLE_TEMPLATES)
        bash_flags, ps_flags, prompt_suffix = random.choice(template["variations"])

        # Build Prompt
        base_prompt = f"{template['intent']} {prompt_suffix}"
        base_prompt = base_prompt.format(target=target, path=path, term=term)
        final_prompt = get_natural_prompt(base_prompt)

        # Build Commands
        bash_cmd = template["bash"].format(
            flags=bash_flags, target=target, path=path, term=term
        )
        ps_cmd = template["ps"].format(
            flags=ps_flags, target=target, path=path, term=term
        )

        # Clean double spaces
        bash_cmd = " ".join(bash_cmd.split())
        ps_cmd = " ".join(ps_cmd.split())

        danger = template.get("dangerous", False)

        return [
            {
                "prompt": final_prompt,
                "command": bash_cmd,
                "dangerous": danger,
                "shell": "bash",
            },
            {
                "prompt": final_prompt,
                "command": ps_cmd,
                "dangerous": danger,
                "shell": "powershell",
            },
        ]


def main():
    print(f"Generating unique examples...")

    unique_hashes = set()
    dataset = []

    while len(dataset) < TARGET_COUNT:
        entries = generate_entry()
        for entry in entries:
            # Create a unique signature for deduplication
            entry_hash = f"{entry['prompt']}_{entry['command']}"

            if entry_hash not in unique_hashes:
                unique_hashes.add(entry_hash)
                dataset.append(entry)

                if len(dataset) % 5000 == 0:
                    print(f"Generated {len(dataset)} items...")

    # Shuffle final dataset
    random.shuffle(dataset)

    # Write to file
    with open(OUTPUT_FILE, "w") as f:
        for entry in dataset:
            f.write(json.dumps(entry) + "\n")

    print(f"Done! Saved {len(dataset)} examples to {OUTPUT_FILE}")


if __name__ == "__main__":
    main()

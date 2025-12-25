import collections
import json

INPUT_FILE = "shell_instruct_dataset_v2.jsonl"


def analyze_dataset():
    total = 0
    shells = collections.Counter()
    dangerous = collections.Counter()

    print(f"Analyzing {INPUT_FILE}...")

    with open(INPUT_FILE, "r") as f:
        for line in f:
            try:
                entry = json.loads(line)
                total += 1
                shells[entry["shell"]] += 1
                dangerous[entry["dangerous"]] += 1

                # integrity check
                if not entry["prompt"] or not entry["command"]:
                    print(f"WARNING: Empty field at line {total}")
            except json.JSONDecodeError:
                print(f"ERROR: Bad JSON at line {total}")

    print(f"\n--- Report ---")
    print(f"Total Examples: {total}")
    print(f"Shell Distribution: {dict(shells)}")
    print(f"Safety Distribution: {dict(dangerous)}")

    # Ideal Ratios:
    # Shells should be roughly 50/50
    # Dangerous should be roughly 10-20% of the dataset

    if shells["bash"] == 0 or shells["powershell"] == 0:
        print("\nCRITICAL: One shell type is missing!")
    else:
        print("\nStatus: READY FOR FINE-TUNING")


if __name__ == "__main__":
    analyze_dataset()

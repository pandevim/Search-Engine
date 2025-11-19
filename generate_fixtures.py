import os
import urllib.parse
import urllib.request
import urllib.error
import json

os.makedirs("fixtures", exist_ok=True)

test_cases = [
    ("01_basic_search", "computer science"),
    ("02_single_term", "algorithm"),
    ("03_empty_query", ""),
    ("04_whitespace_query", "   "),
    ("05_no_results", "supercalifragilisticexpialidocious_nonexistent_term"),
    ("06_special_chars", "c++"),
    ("07_stopwords_only", "the a an"),
    ("08_case_insensitive", "CoMpUtEr"),
    ("09_intersection", "computer algorithm"),
]

base_url = "http://127.0.0.1:8080/search"

print(f"Generating fixtures in {os.path.abspath('fixtures')}...")

for name, query in test_cases:
    print(f"Running test: {name} (query: '{query}')")
    
    # Save input
    with open(f"fixtures/{name}_input.txt", "w") as f:
        f.write(query)
        
    # Prepare URL
    params = urllib.parse.urlencode({'q': query})
    url = f"{base_url}?{params}"
    
    try:
        with urllib.request.urlopen(url) as response:
            data = json.loads(response.read().decode())
            
            # Save output
            with open(f"fixtures/{name}_output.json", "w") as f:
                json.dump(data, f, indent=2)
                
    except urllib.error.HTTPError as e:
        print(f"  -> HTTP {e.code} received (expected for some boundary conditions)")
        # The server returns JSON even for 400 Bad Request
        error_body = e.read().decode()
        try:
            data = json.loads(error_body)
            with open(f"fixtures/{name}_output.json", "w") as f:
                json.dump(data, f, indent=2)
        except json.JSONDecodeError:
            # Fallback if not JSON
            with open(f"fixtures/{name}_output.txt", "w") as f:
                f.write(f"HTTP {e.code}\n{error_body}")
    except Exception as e:
        print(f"  -> Error: {e}")

print("Fixtures generation complete.")

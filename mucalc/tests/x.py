import sys, subprocess, os

op = sys.argv[1]

if op == "aut":
    f = sys.argv[2]
    subprocess.call(f'mcrl22lps "{f}.mcrl2" "{f}.lps" --timings')
    subprocess.call(f'lps2lts "{f}.lps" "{f}.lts" --timings')
    subprocess.call(f'ltsconvert "{f}.lts" "{f}.aut" --timings')
    os.remove(f"{f}.lps")
    os.remove(f"{f}.lts")
elif op == "solve":
    f = sys.argv[2]
    g = sys.argv[3]
    subprocess.call(f'mcrl22lps "{f}.mcrl2" "{f}.lps" --timings')
    subprocess.call(f'lps2pbes --formula="{g}.mcf" "{f}.lps" "{g}.pbes" --timings')
    subprocess.call(f'pbes2bool -rjitty "{g}.pbes" --timings')
    os.remove(f"{f}.lps")
    os.remove(f"{g}.pbes")
else:
    print("Invalid command")

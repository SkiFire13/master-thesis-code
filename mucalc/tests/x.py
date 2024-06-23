import sys, subprocess, os

dir = os.path.dirname(os.path.realpath(__file__))
os.chdir(dir)

op = sys.argv[1]

if op == "aut":
    base = sys.argv[2]
    f = f"{base}/{base}"
    subprocess.call(f'mcrl22lps "{f}.mcrl2" "{f}.lps" --timings')
    subprocess.call(f'lps2lts "{f}.lps" "{f}.lts" --timings')
    subprocess.call(f'ltsconvert "{f}.lts" "{f}.aut" --timings')
    os.remove(f"{f}.lps")
    os.remove(f"{f}.lts")
elif op == "solve":
    base = sys.argv[2]
    formula = sys.argv[3]
    f = f"{base}/{base}"
    g = f"{base}/{formula}"
    subprocess.call(f'mcrl22lps "{f}.mcrl2" "{f}.lps" --timings')
    subprocess.call(f'lps2pbes --formula="{g}.mcf" "{f}.lps" "{g}.pbes" --timings')
    subprocess.call(f'pbes2bool -rjitty "{g}.pbes" --timings')
    os.remove(f"{f}.lps")
    os.remove(f"{g}.pbes")
else:
    print("Invalid command")

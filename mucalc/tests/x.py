import sys, subprocess, os

dir = os.path.dirname(os.path.realpath(__file__))
os.chdir(dir)

op = sys.argv[1]

if op == "aut":
    base = sys.argv[2]
    f = f"{base}/{base}"
    subprocess.run(['./mcrl22lps', f'{f}.mcrl2', f'{f}.lps' '--timings'])
    subprocess.run(['./lps2lts', f'{f}.lps', f'{f}.lts', '--timings'])
    subprocess.run(['./ltsconvert', f'{f}.lts', f'{f}.aut', '--timings'])
    os.remove(f"{f}.lps")
    os.remove(f"{f}.lts")
elif op == "solve":
    base = sys.argv[2]
    formula = sys.argv[3]
    f = f"{base}/{base}"
    g = f"{base}/{formula}"
    subprocess.run(['./mcrl22lps', f'{f}.mcrl2', f'{f}.lps', '--timings'])
    subprocess.run(['./lps2pbes', f'--formula={g}.mcf', f'{f}.lps', f'{g}.pbes', '--timings'])
    subprocess.run(['./pbes2bool', '-rjitty', f'{g}.pbes', '--timings'])
    os.remove(f"{f}.lps")
    os.remove(f"{g}.pbes")
else:
    print("Invalid command")

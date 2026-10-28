#!/usr/bin/env python3
"""Deterministic protocol fixture for E2E only. Never a production engine fallback."""
import json
import sys
import time

if 'version' in sys.argv:
    print('KataGo protocol fixture\nUsing deterministic test backend')
    sys.exit(0)

columns='ABCDEFGHJKLMNOPQRST'
def point(s):
    return (19-int(s[1:]))*19+columns.index(s[0])
def coordinate(p):
    return columns[p%19]+str(19-p//19)
for line in sys.stdin:
    q=json.loads(line)
    if q.get('action')=='query_models':
        models=[dict(name='strong-fixture',usesHumanSLProfile=False,version=14)]
        if '-human-model' in sys.argv:models.append(dict(name='human-fixture',usesHumanSLProfile=True,version=15))
        print(json.dumps(dict(id=q['id'],action=q['action'],models=models)),flush=True)
        continue
    board=[0]*361
    for color,move in q['moves']:
        if move.lower()!='pass':board[point(move)]=1 if color=='B' else 2
    preferred=next((i for i in [300,60,72,288,*range(361)] if not board[i]),361)
    move='pass' if q['moves'] and q['moves'][-1][1].lower()=='pass' else coordinate(preferred)
    visits=q['maxVisits']+1
    policy=[-1. if s else 0. for s in board]+[0.]
    # Human policy deliberately never passes: search-based ending must override it.
    policy[preferred]=1.
    candidates=[dict(move=move,visits=visits,scoreLead=-2.,winrate=.45,pv=[move],order=0)]
    if q.get('includePolicy'):time.sleep(.12)
    print(json.dumps(dict(id=q['id'],isDuringSearch=False,rootInfo=dict(visits=visits,scoreLead=-2.,scoreStdev=10.,winrate=.45),moveInfos=candidates,humanPolicy=policy,ownership=[1. if s==1 else -1. if s==2 else 0. for s in board])),flush=True)

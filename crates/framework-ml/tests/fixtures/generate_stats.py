"""Independent scipy references; run with the pinned statistics reference environment."""
import json
from pathlib import Path
import numpy as np
import scipy
from scipy import stats

x=np.array([3.2,1.1,4.7,2.8,5.9,2.4,6.1,4.0])
y=np.array([8.2,2.5,7.1,4.2,8.0,6.3,7.6,5.2])
y_welch=np.array([1.8,2.2,3.5,1.1,4.4,2.9])
results=[]
for level in [0.90,0.95]:
    se=stats.sem(x)
    low,high=stats.t.interval(level,df=len(x)-1,loc=x.mean(),scale=se)
    results.append(dict(method="meanConfidence",x=x.tolist(),y=None,confidenceLevel=level,estimate=float(x.mean()),standardError=float(se),degreesOfFreedom=float(len(x)-1),confidenceLower=float(low),confidenceUpper=float(high)))
    result=stats.pearsonr(x,y)
    ci=result.confidence_interval(confidence_level=level)
    results.append(dict(method="pearsonCorrelation",x=x.tolist(),y=y.tolist(),confidenceLevel=level,estimate=float(result.statistic),pValue=float(result.pvalue),confidenceLower=float(ci.low),confidenceUpper=float(ci.high)))
    result=stats.ttest_ind(x,y_welch,equal_var=False)
    ci=result.confidence_interval(confidence_level=level)
    results.append(dict(method="welchDifference",x=x.tolist(),y=y_welch.tolist(),confidenceLevel=level,estimate=float(x.mean()-y_welch.mean()),statistic=float(result.statistic),degreesOfFreedom=float(result.df),pValue=float(result.pvalue),confidenceLower=float(ci.low),confidenceUpper=float(ci.high)))
Path(__file__).with_name("stats.json").write_text(json.dumps(dict(scipy=scipy.__version__,cases=results),indent=2)+"\n")

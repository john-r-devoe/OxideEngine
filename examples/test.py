#! Simple bar sanity test

import oxide_engine as oxide
from datetime import datetime

data = oxide.data_loader.from_csv("C:/Users/johnd/Documents/OxideEngine/examples/AMZN_1min_sample.csv", "AMZN")

#print(data)
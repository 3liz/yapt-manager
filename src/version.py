# 
# Check QGIS version
#
import sys
try:
    from qgis.core import Qgis
except ModuleNotFoundError:
    # No qgis installed bail out
    print("", end='')
else:
    print(Qgis.QGIS_VERSION.split("-")[0], end = '')

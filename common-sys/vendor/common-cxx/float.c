/* *********************************************************************
 * This Original Work is copyright of 51 Degrees Mobile Experts Limited.
 * Copyright 2026 51 Degrees Mobile Experts Limited, Davidson House,
 * Forbury Square, Reading, Berkshire, United Kingdom RG1 3EU.
 *
 * This Original Work is licensed under the European Union Public Licence
 * (EUPL) v.1.2 and is subject to its terms as set out below.
 *
 * If a copy of the EUPL was not distributed with this file, You can obtain
 * one at https://opensource.org/licenses/EUPL-1.2.
 *
 * The 'Compatible Licences' set out in the Appendix to the EUPL (as may be
 * amended by the European Commission) shall be deemed incompatible for
 * the purposes of the Work and the provisions of the compatibility
 * clause in Article 5 of the EUPL shall not apply.
 *
 * If using the Work as, or as part of, a network application, by
 * including the attribution notice(s) required under Article 5 of the EUPL
 * in the end user terms of the application under an appropriate heading,
 * such notice(s) shall fulfill the requirements of that article.
 * ********************************************************************* */

#include <math.h>
#include "fiftyone.h"

float fiftyoneDegreesFloatToNative(fiftyoneDegreesFloatInternal f) {
	fiftyoneDegreesFloatU floatU;
	float nativeValue;

	floatU.fValue = f;
	if (floatU.parts.exponent == 0) {
		if (floatU.parts.mantissa == 0) {
			// When all bits in exponent and mantissa are 0s. The float value is 0.
			nativeValue = 0;
		}
		else {
			/*
			 * When all bits in exponent are 0s but not in the mantissa. This is a
			 * denormalised float (or subnormal case).
			 * The exponent will be treated as '00000001' instead and the mantissa
			 * won't use the hidden bit.
			 * i.e. float = (-1)^sign * 2^(1 - bias) * (0 + mantissa)
			 */
			nativeValue = (float)powf(-1, (float)floatU.parts.sign);
			nativeValue *= powf(FIFTYONE_DEGREES_FLOAT_RADIX, 1 - FIFTYONE_DEGREES_FLOAT_BIAS);
			/*
			 * The actual mantissa value is calculated by take its unsigned integer
			 * and divided by 2 to the power of number of bits in the mantissa.
			 * This is how the fractional point is shifted in binary number.
			 */
			nativeValue *= ((float)floatU.parts.mantissa) / powf(FIFTYONE_DEGREES_FLOAT_RADIX, FIFTYONE_DEGREES_FLOAT_MANT_SIZE);
		}
	}
	else if (floatU.parts.exponent == FIFTYONE_DEGREES_FLOAT_EXP_MAX) {
		if (floatU.parts.mantissa == FIFTYONE_DEGREES_FLOAT_MANT_MAX) {
			/*
			 * If all bits in exponent and mantissa are 1s, it is an infinity value
			 */
			nativeValue = INFINITY;
		}
		else {
			/*
			 * If all bits in exponet are 1s but not the mantissa, it is not a number
			 */
			nativeValue = NAN;
		}
	}
	else {
		/*
		 * Normal case. Float value is caluclated by
		 * float = (-1)^sign * 2^(exponent - bias) * (1 + mantissa)
		 */
		nativeValue = (float)powf(-1, (float)floatU.parts.sign);
		nativeValue *= (floatU.parts.exponent == 0 ? 0 : powf(FIFTYONE_DEGREES_FLOAT_RADIX, (float)(floatU.parts.exponent) - FIFTYONE_DEGREES_FLOAT_BIAS));
		nativeValue *= (1 + ((float)floatU.parts.mantissa) / powf(FIFTYONE_DEGREES_FLOAT_RADIX, FIFTYONE_DEGREES_FLOAT_MANT_SIZE));
	}
	return nativeValue;
}

fiftyoneDegreesFloatInternal fiftyoneDegreesNativeToFloat(float f) {
	fiftyoneDegreesFloatU floatU = { .fValue = 0 };
	float significand;
	int exponent;

	switch (fpclassify(f)) {
	case FP_NAN:
		// If NaN, set exponent to all 1s and mantissa to 1
		floatU.parts.exponent = FIFTYONE_DEGREES_FLOAT_EXP_MAX;
		floatU.parts.mantissa = 1;
		break;
	case FP_INFINITE:
		// If Inf, set exponent and mantissa to all 1s
		floatU.parts.exponent = FIFTYONE_DEGREES_FLOAT_EXP_MAX;
		floatU.parts.mantissa = FIFTYONE_DEGREES_FLOAT_MANT_MAX;
		break;
	case FP_SUBNORMAL:
		// If subnormal, leave exponent to 0
		significand = (float)frexp(f, &exponent);
		floatU.parts.sign = (f >= 0) ? 0 : -1;
		/*
		 * Significand is fractional so time with 2 to power of number of mantissa bits
		 * to get its integer
		 */
		floatU.parts.mantissa = (int)(fabsf(significand) * powf(FIFTYONE_DEGREES_FLOAT_RADIX, FIFTYONE_DEGREES_FLOAT_MANT_SIZE));
		break;
	case FP_NORMAL:
		significand = (float)frexp(f, &exponent);
		floatU.parts.sign = (f >= 0) ? 0 : -1;
		/*
		 * frexp returns value between (-1, 0.5],[0.5, 1) * exponent of 2
		 * to convert to the float calculation formular
		 * (-1)^sign + 2^(exponent - bias) * (1 + mantissa)
		 * the significand need to be > 1. Thus move 1 bit from the exponent
		 * to the mantissa. This 1 bit represents the hidden bit.
		 */
		floatU.parts.exponent = exponent - 1 + FIFTYONE_DEGREES_FLOAT_BIAS;
		floatU.parts.mantissa = (int)((fabsf(significand) * FIFTYONE_DEGREES_FLOAT_RADIX - 1)
			* powf(FIFTYONE_DEGREES_FLOAT_RADIX, FIFTYONE_DEGREES_FLOAT_MANT_SIZE));
		break;
	case FP_ZERO:
	default:
		break;
	}
	return floatU.fValue;
}

int fiftyoneDegreesFloatIsEqual(fiftyoneDegreesFloatInternal f1, fiftyoneDegreesFloatInternal f2) {
	int isEqual = 0;
	for (int i = 0; i < FIFTYONE_DEGREES_FLOAT_SIZE; i++) {
		isEqual |= (f1.value[i] == f2.value[i] ? 0 : 1);
	}
	return isEqual;
}
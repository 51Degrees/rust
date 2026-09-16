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

#ifndef FIFTYONE_DEGREES_WKBTOT_H_INCLUDED
#define FIFTYONE_DEGREES_WKBTOT_H_INCLUDED

#include <stdbool.h>

#include "string.h"
#include "exceptions.h"

/**
 * Used as a return type from the conversion routines to carry information about
 * the operation results to the caller, allows the caller to f.e. judge about the buffer utilization,
 * and whether the buffer was of sufficient size
 */
typedef struct fiftyone_degrees_transform_wkb_to_t_result {
	/**
	 * number of characters written or that would have been written to the buffer, reflects required buffer size
	 */
	size_t written;

	/**
	 * the caller should check this flag and reallocate the buffer to be of at least `written` size
	 * if this flag is set
	 */
	bool bufferTooSmall;
} fiftyoneDegreesWkbtotResult;

typedef enum {
	FIFTYONE_DEGREES_WKBToT_REDUCTION_NONE = 0, /**< Standard compliant */
	FIFTYONE_DEGREES_WKBToT_REDUCTION_SHORT = 1, /**< Some values reduced to int16_t */
} fiftyoneDegreesWkbtotReductionMode;

/**
 * Converts WKB geometry bytes to WKT string and writes it to string builder.
 * @param wellKnownBinary bytes of WKB geometry.
 * @param reductionMode type/value reduction applied to decrease WKB size.
 * @param decimalPlaces precision for numbers (places after the decimal dot).
 * @param builder string builder to write WKT into.
 * @param exception pointer to the exception struct.
 * @return How many bytes were written to the buffer and if it was too small.
 */
EXTERNAL void
fiftyoneDegreesWriteWkbAsWktToStringBuilder
(const unsigned char *wellKnownBinary,
 fiftyoneDegreesWkbtotReductionMode reductionMode,
 uint8_t decimalPlaces,
 fiftyoneDegreesStringBuilder *builder,
 fiftyoneDegreesException *exception);

/**
 * Converts WKB geometry bytes to WKT string written into provided buffer.
 * @param wellKnownBinary bytes of WKB geometry.
 * @param reductionMode type/value reduction applied to decrease WKB size.
 * @param buffer buffer to write WKT geometry into.
 * @param length length available in the buffer.
 * @param decimalPlaces precision for numbers (places after the decimal dot).
 * @param exception pointer to the exception struct.
 * @return How many bytes were written to the buffer and if it was too small.
 */
EXTERNAL fiftyoneDegreesWkbtotResult
fiftyoneDegreesConvertWkbToWkt
(const unsigned char *wellKnownBinary,
 fiftyoneDegreesWkbtotReductionMode reductionMode,
 char *buffer, size_t length,
 uint8_t decimalPlaces,
 fiftyoneDegreesException *exception);

#endif //FIFTYONE_DEGREES_WKBTOT_H_INCLUDED

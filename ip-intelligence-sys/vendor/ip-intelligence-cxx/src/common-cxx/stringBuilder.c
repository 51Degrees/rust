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

#define __STDC_FORMAT_MACROS

#include "stringBuilder.h"
#include "fiftyone.h"
#include <inttypes.h>

/**
 * Add IPv4 address (raw bytes) to string builder (as text)
 * @param ipAddress raw bytes of IPv4
 * @param stringBuilder destination
 */
static void getIpv4RangeString(
	const unsigned char * const ipAddress,
	StringBuilder * const stringBuilder) {
	StringBuilderAddInteger(stringBuilder, ipAddress[0]);
	StringBuilderAddChar(stringBuilder, '.');
	StringBuilderAddInteger(stringBuilder, ipAddress[1]);
	StringBuilderAddChar(stringBuilder, '.');
	StringBuilderAddInteger(stringBuilder, ipAddress[2]);
	StringBuilderAddChar(stringBuilder, '.');
	StringBuilderAddInteger(stringBuilder, ipAddress[3]);
}

/**
 * Add IPv6 address (raw bytes) to string builder (as text)
 * @param ipAddress raw bytes of IPv6
 * @param stringBuilder destination
 */
static void getIpv6RangeString(
	const unsigned char * const ipAddress,
	StringBuilder * const stringBuilder) {
	const char separator = ':';
	const char *hex = "0123456789abcdef";
	for (int i = 0; i < FIFTYONE_DEGREES_IPV6_LENGTH; i += 2) {
		for (int j = 0; j < 2; j++) {
			StringBuilderAddChar(stringBuilder, hex[(((int)ipAddress[i + j]) >> 4) & 0x0F]);
			StringBuilderAddChar(stringBuilder, hex[((int)ipAddress[i + j]) & 0x0F]);
		}
		if (i != FIFTYONE_DEGREES_IPV6_LENGTH - 2) {
			StringBuilderAddChar(stringBuilder, separator);
		}
	}
}

void fiftyoneDegreesStringBuilderAddIpAddress(
	StringBuilder * const stringBuilder,
	const VarLengthByteArray * const ipAddress,
	const IpType type,
	Exception * const exception) {
	const int32_t ipLength =
		type == IP_TYPE_IPV4 ?
		FIFTYONE_DEGREES_IPV4_LENGTH :
		FIFTYONE_DEGREES_IPV6_LENGTH;
	// Get the actual length of the byte array
	int32_t actualLength = ipAddress->size;

	// Make sure the ipAddress item and everything is in correct
	// format
	if (ipLength == actualLength
		&& type != IP_TYPE_INVALID) {

		if (type == IP_TYPE_IPV4) {
			getIpv4RangeString(
				&ipAddress->firstByte,
				stringBuilder);
		}
		else {
			getIpv6RangeString(
				&ipAddress->firstByte,
				stringBuilder);
		}
	}
	else {
		EXCEPTION_SET(INCORRECT_IP_ADDRESS_FORMAT);
	}
}

fiftyoneDegreesStringBuilder* fiftyoneDegreesStringBuilderInit(
	fiftyoneDegreesStringBuilder* builder) {
	builder->current = builder->ptr;
	builder->remaining = builder->length;
	builder->added = 0;
	builder->full = false;
	return builder;
}

fiftyoneDegreesStringBuilder* fiftyoneDegreesStringBuilderAddChar(
	fiftyoneDegreesStringBuilder* builder,
	char const value) {
	if (builder->remaining > 1) {
		*builder->current = value;
		builder->current++;
		builder->remaining--;
	}
	else {
		builder->full = true;
	}
	builder->added++;
	return builder;
}

fiftyoneDegreesStringBuilder* fiftyoneDegreesStringBuilderAddInteger(
	fiftyoneDegreesStringBuilder* builder,
	int64_t const value) {
    // 64-bit INT_MIN is  -9,223,372,036,854,775,807 => 21 characters
	char temp[22];
	if (snprintf(temp, sizeof(temp), "%" PRId64, value) > 0) {
		StringBuilderAddChars(
			builder,
			temp,
			strlen(temp));
	}
	return builder;
}

StringBuilder* fiftyoneDegreesStringBuilderAddDouble(
	fiftyoneDegreesStringBuilder * const builder,
	const double value,
	const uint8_t decimalPlaces) {
	bool addNegative = false;
	const int digitPlaces = MAX_DOUBLE_DECIMAL_PLACES < decimalPlaces
		? MAX_DOUBLE_DECIMAL_PLACES : decimalPlaces;
	int remDigits = digitPlaces;

	int intPart = (int)value;
	double fracPart = value - intPart;

	if (fracPart < 0) {
		if (intPart == 0) {
			// Handle negative numbers <1. The integer part will just be zero,
			// which is neither positive or negative. So the negative must be
			// added.
			addNegative = true;
		}
		fracPart = -fracPart;
	}

	if (!fracPart || remDigits <= 0) {
		if (fracPart >= 0.5) {
			if (value < 0) {
				intPart--;
			} else {
				intPart++;
			}
		}
		StringBuilderAddInteger(builder, intPart);
		return builder;
	}

	char floatTail[MAX_DOUBLE_DECIMAL_PLACES + 2];
	floatTail[0] = '.';
	char *digits = floatTail + 1;

	char *nextDigit = digits;
	while (remDigits > 0 && fracPart) {
		remDigits--;
		fracPart *= 10;
		*nextDigit = (char)fracPart;
		fracPart -= *nextDigit;
		if (!remDigits && fracPart >= 0.5) {
			// find last non-9
			while (nextDigit > floatTail && *nextDigit == 9) {
				--nextDigit;
				++remDigits;
			}
			if (nextDigit > floatTail) {
				(*nextDigit)++;
				++nextDigit;
				break;
			} else {
				// tail collapsed into 1
				if (value < 0) {
					intPart--;
				} else {
					intPart++;
				}
				StringBuilderAddInteger(builder, intPart);
				return builder;
			}
		}
		++nextDigit;
	}
	*nextDigit = '\0';

	int digitsToAdd = digitPlaces - remDigits;
	for (nextDigit = digits + digitsToAdd - 1;
		nextDigit >= digits;
		--nextDigit) {

		if (*nextDigit) {
			break;
		}
		--digitsToAdd;
	}
	if (digitsToAdd <= 0) {
		// tail collapsed to 0
		StringBuilderAddInteger(builder, intPart);
		return builder;
	}
	for (; nextDigit >= digits; --nextDigit) {
		*nextDigit += '0';
	}
	if (addNegative) {
		StringBuilderAddChar(builder, '-');
	}
	StringBuilderAddInteger(builder, intPart);
	StringBuilderAddChars(builder, floatTail, digitsToAdd + 1);
	return builder;
}

fiftyoneDegreesStringBuilder* fiftyoneDegreesStringBuilderAddChars(
	fiftyoneDegreesStringBuilder* builder,
	const char * const value,
	size_t const length) {
	const bool fitsIn = length < builder->remaining;
	const size_t clippedLength = (
		fitsIn ? length : (builder->remaining ? builder->remaining - 1 : 0));
	if (0 < clippedLength &&
		memcpy(builder->current, value, clippedLength) == builder->current) {
		builder->remaining -= clippedLength;
		builder->current += clippedLength;
	}
	if (!fitsIn) {
		builder->full = true;
	}
	builder->added += length;
	return builder;
}

StringBuilder* fiftyoneDegreesStringBuilderAddStringValue(
	fiftyoneDegreesStringBuilder * const builder,
	const fiftyoneDegreesStoredBinaryValue* const value,
	fiftyoneDegreesPropertyValueType const valueType,
	const uint8_t decimalPlaces,
	fiftyoneDegreesException * const exception) {

	switch (valueType) {
		case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_IP_ADDRESS: {
			// Get the actual address size
			const uint16_t addressSize = value->byteArrayValue.size;
			// Get the type of the IP address
			fiftyoneDegreesIpType type;
			switch (addressSize) {
				case FIFTYONE_DEGREES_IPV4_LENGTH: {
					type = IP_TYPE_IPV4;
					break;
				}
				case FIFTYONE_DEGREES_IPV6_LENGTH: {
					type = IP_TYPE_IPV6;
					break;
				}
				default: {
					type = IP_TYPE_INVALID;
					break;
				}
			}
			// Get the string representation of the IP address
			StringBuilderAddIpAddress(
				builder,
				&value->byteArrayValue,
				type,
				exception);
			break;
		}
		case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_WKB: {
			fiftyoneDegreesWriteWkbAsWktToStringBuilder(
				&value->byteArrayValue.firstByte,
				FIFTYONE_DEGREES_WKBToT_REDUCTION_NONE,
				decimalPlaces,
				builder,
				exception);
			break;
		}
		case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_WKB_R: {
			fiftyoneDegreesWriteWkbAsWktToStringBuilder(
				&value->byteArrayValue.firstByte,
				FIFTYONE_DEGREES_WKBToT_REDUCTION_SHORT,
				decimalPlaces,
				builder,
				exception);
			break;
		}
		case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING: {
			// discard NUL-terminator
			if (value->stringValue.size > 1) {
				StringBuilderAddChars(
					builder,
					&value->stringValue.value,
					value->stringValue.size - 1);
			}
			break;
		}
		case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_AZIMUTH:
		case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_DECLINATION:
		case FIFTYONE_DEGREES_PROPERTY_VALUE_SINGLE_PRECISION_FLOAT: {
			StringBuilderAddDouble(
				builder,
				StoredBinaryValueToDoubleOrDefault(
					value,
					valueType,
					0),
				decimalPlaces);
			break;
		}
		case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_INTEGER: {
			StringBuilderAddInteger(builder, value->intValue);
			break;
		}
		case FIFTYONE_DEGREES_PROPERTY_VALUE_SINGLE_BYTE: {
			StringBuilderAddInteger(builder, value->byteValue);
			break;
		}
		default: {
			EXCEPTION_SET(UNSUPPORTED_STORED_VALUE_TYPE);
			break;
		}
	}

	return builder;
}

fiftyoneDegreesStringBuilder* fiftyoneDegreesStringBuilderComplete(
	fiftyoneDegreesStringBuilder* builder) {

	// Always ensures that the string is null terminated even if that means
	// overwriting the last character to turn it into a null.
	if (builder->remaining >= 1) {
		*builder->current = '\0';
		builder->current++;
		builder->remaining--;
		builder->added++;
	}
	else {
        if (builder->ptr && builder->length > 0) {
            *(builder->ptr + builder->length - 1) = '\0';
        }
		builder->full = true;
	}
	return builder;
}

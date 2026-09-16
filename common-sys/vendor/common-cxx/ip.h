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

#ifndef FIFTYONE_DEGREES_IP_H_INCLUDED
#define FIFTYONE_DEGREES_IP_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup fiftyoneDegreesIp IP
 *
 * Types and methods to parse IP address strings.
 *
 * ## Introduction
 *
 * IP v4 and v6 addresses can be parsed using the
 * #fiftyoneDegreesIpAddressParse and #fiftyoneDegreesIpAddressesParse methods.
 *
 * @{
 */

#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include "data.h"
#include "common.h"

/**
 * The number of bytes in an Ipv4 Address
 */
#define FIFTYONE_DEGREES_IPV4_LENGTH 4

/**
 * The number of bytes in an Ipv6 Address
 */
#define FIFTYONE_DEGREES_IPV6_LENGTH 16

/**
 * Enum indicating the type of IP address.
 */
typedef enum e_fiftyone_degrees_ip_type {
	FIFTYONE_DEGREES_IP_TYPE_INVALID = 0, /**< Invalid IP address */
	FIFTYONE_DEGREES_IP_TYPE_IPV4 = 4, /**< An IPv4 address */
	FIFTYONE_DEGREES_IP_TYPE_IPV6 = 6, /**< An IPv6 address */
} fiftyoneDegreesIpType;

/**
 * The structure to hold a IP Address in byte array format.
 */
typedef struct fiftyone_degrees_ip_address_t {
	byte value[FIFTYONE_DEGREES_IPV6_LENGTH]; /**< Buffer to hold the IP 
											  address bytes array. */
	byte type; /**< The type of the IP. @see fiftyoneDegreesIpType */
} fiftyoneDegreesIpAddress;

/**
 * Parse a single IP address string.
 * Does not modify the last 12 (out of 16) bytes when parsing IPv4.
 * @param start of the string containing the IP address to parse
 * @param end the last character of the string (with IP address) to be considered for parsing
 * @param address memory to write parsed IP address into.
 * @return <c>true</c> if address was parsed correctly, <c>false</c> otherwise
 */
EXTERNAL bool fiftyoneDegreesIpAddressParse(
	const char *start,
	const char *end,
	fiftyoneDegreesIpAddress *address);

/**
 * Compare two IP addresses in its binary form
 * @param ipAddress1 the first IP address
 * @param ipAddress2 the second IP address
 * @param type the type of IP address. This determine
 * the number of bytes to compare. IPv4 require 4 bytes
 * and IPv6 require 16 bytes
 * @return a value indicate the result:
 * 0 for equals
 * > 0 for ipAddress1 comes after ipAddress2
 * < 0 for ipAddress1 comes before ipAddress2
 */
EXTERNAL int fiftyoneDegreesIpAddressesCompare(
	const unsigned char *ipAddress1,
	const unsigned char *ipAddress2,
	fiftyoneDegreesIpType type);

/**
 * @}
 */

#endif

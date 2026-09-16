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

#ifndef FIFTYONE_DEGREES_RESULTS_INCLUDED
#define FIFTYONE_DEGREES_RESULTS_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesResults Results
 *
 * Structure returned by an engine's process method(s), containing values.
 *
 * ## Introduction
 *
 * Results are the structure returned by an engine's process method(s) and
 * contains the values for the properties in the engine.
 * The base structure contains only the essential element of a data set, which
 * is needed to return values. Any engine can extend this structure to add any
 * additional elements which may be needed to format or return values.
 *
 * The data set pointer is a managed resource, meaning that while the results
 * exist, the data set will not be freed by the resource manager. This ensures
 * that values contained in the data set used for processing can always be
 * returned.
 *
 * @{
 */

#include "data.h"
#include "status.h"
#include "dataset.h"
#include "common.h"

/**
 * Enum containing reasons which cause a value to not be present or valid.
 */
typedef enum e_fiftyone_degrees_results_no_value_reason {
	FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_DIFFERENCE, /**< The difference
														 value is higher than
														 the threshold, see
														 the Pattern API */
	FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_NO_MATCHED_NODES, /**< No hash
															   nodes were
															   matched, see the
															   Hash API */
	FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_INVALID_PROPERTY, /**< The requested
															   property does
															   not exist, or is
															   not a required
															   property */
	FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_NO_RESULT_FOR_PROPERTY, /**< There
																	 is no result
																	 which
																	 contains a
																	 value for
																	 the requested
																	 property */
	FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_NO_RESULTS, /**< There are no
														 results to get a value
														 from */
	FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_TOO_MANY_VALUES, /**< There are too
															  many values to be
															  expressed as the
															  requested type */
	FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_NULL_PROFILE, /**< The results
														   contain a null
														   profile for the
														   required component */
	FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_HIGH_RISK, /**< The match is
	                                                    deemed high risk of
														containing incorrect or
														misleading results. */
	FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_UNKNOWN /**< None of the above */
} fiftyoneDegreesResultsNoValueReason;

/**
 * Base results structure which any processing results should extend.
 */
typedef struct fiftyone_degrees_results_base_t {
	void *dataSet; /**< Pointer to the data set associated with the results */
} fiftyoneDegreesResultsBase;

/**
 * Initialise a set of results by setting the data set they are associated with.
 * @param results pointer to the results to initialise
 * @param dataSet pointer to the data set which will be using the results
 * @return pointer to the initialised results
 */
EXTERNAL fiftyoneDegreesResultsBase* fiftyoneDegreesResultsInit(
	fiftyoneDegreesResultsBase *results,
	void *dataSet);

/**
 * @}
 */

#endif
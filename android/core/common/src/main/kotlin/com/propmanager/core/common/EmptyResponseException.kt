package com.propmanager.core.common

class EmptyResponseException(endpoint: String = "API") :
    RuntimeException("Empty response from $endpoint")

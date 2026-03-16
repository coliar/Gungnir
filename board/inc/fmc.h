/**
    ******************************************************************************
    * File Name          : FMC.h
    * Description        : This file provides code for the configuration
    *                      of the FMC peripheral.
    ******************************************************************************
    */
#ifndef __FMC_H
#define __FMC_H
#ifdef __cplusplus
 extern "C" {
#endif

#include "board.h"

extern SDRAM_HandleTypeDef hsdram1;

void MX_FMC_Init(void);
void HAL_SDRAM_MspInit(SDRAM_HandleTypeDef* hsdram);
void HAL_SDRAM_MspDeInit(SDRAM_HandleTypeDef* hsdram);


#ifdef __cplusplus
}
#endif
#endif /*__FMC_H */
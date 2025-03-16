#include "sdmmc.h"
#include "printf.h"
#include <stdint.h>

#define SD_TIMEOUT             ((uint32_t)0x00100000U)



SD_HandleTypeDef SDHandle;
HAL_SD_CardInfoTypeDef SDCardInfo;


static uint8_t Wait_SDCARD_Ready(void) {
  uint32_t loop = SD_TIMEOUT;
  
  while(loop > 0) {
    loop--;
    if(HAL_SD_GetCardState(&SDHandle) == HAL_SD_CARD_TRANSFER) {
      return HAL_OK;
    }
  }
  return HAL_ERROR;
}

void HAL_SD_MspInit(SD_HandleTypeDef* sdHandle) {
  GPIO_InitTypeDef GPIO_InitStruct = {0};
  RCC_PeriphCLKInitTypeDef PeriphClkInitStruct = {0};

  if(sdHandle->Instance == SDMMC2) {
    PeriphClkInitStruct.PeriphClockSelection = RCC_PERIPHCLK_SDMMC;
    PeriphClkInitStruct.SdmmcClockSelection = RCC_SDMMCCLKSOURCE_PLL;
    if (HAL_RCCEx_PeriphCLKConfig(&PeriphClkInitStruct) != HAL_OK) {
      Error_Handler();
    }

    /* SDMMC2 clock enable */
    __HAL_RCC_SDMMC2_CLK_ENABLE();

    __HAL_RCC_GPIOB_CLK_ENABLE();
    __HAL_RCC_GPIOD_CLK_ENABLE();
    /**SDMMC2 GPIO Configuration
    PB14     ------> SDMMC2_D0
    PB15     ------> SDMMC2_D1
    PD6     ------> SDMMC2_CK
    PD7     ------> SDMMC2_CMD
    PB3 (JTDO/TRACESWO)     ------> SDMMC2_D2
    PB4 (NJTRST)     ------> SDMMC2_D3
    */
    GPIO_InitStruct.Pin = GPIO_PIN_14|GPIO_PIN_15|GPIO_PIN_3|GPIO_PIN_4;
    GPIO_InitStruct.Mode = GPIO_MODE_AF_PP;
    GPIO_InitStruct.Pull = GPIO_NOPULL;
    GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_VERY_HIGH;
    GPIO_InitStruct.Alternate = GPIO_AF9_SDIO2;
    HAL_GPIO_Init(GPIOB, &GPIO_InitStruct);

    GPIO_InitStruct.Pin = GPIO_PIN_6|GPIO_PIN_7;
    GPIO_InitStruct.Mode = GPIO_MODE_AF_PP;
    GPIO_InitStruct.Pull = GPIO_NOPULL;
    GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_VERY_HIGH;
    GPIO_InitStruct.Alternate = GPIO_AF11_SDIO2;
    HAL_GPIO_Init(GPIOD, &GPIO_InitStruct);

    /* SDMMC2 interrupt Init */
    HAL_NVIC_SetPriority(SDMMC2_IRQn, 0, 0);
    HAL_NVIC_EnableIRQ(SDMMC2_IRQn);
  }
}

void HAL_SD_MspDeInit(SD_HandleTypeDef* sdHandle) {
  if(sdHandle->Instance == SDMMC2) {
    __HAL_RCC_SDMMC2_CLK_DISABLE();

    /**SDMMC2 GPIO Configuration
    PB14     ------> SDMMC2_D0
    PB15     ------> SDMMC2_D1
    PD6     ------> SDMMC2_CK
    PD7     ------> SDMMC2_CMD
    PB3 (JTDO/TRACESWO)     ------> SDMMC2_D2
    PB4 (NJTRST)     ------> SDMMC2_D3
    */
    HAL_GPIO_DeInit(GPIOB, GPIO_PIN_14|GPIO_PIN_15|GPIO_PIN_3|GPIO_PIN_4);

    HAL_GPIO_DeInit(GPIOD, GPIO_PIN_6|GPIO_PIN_7);

    /* SDMMC2 interrupt Deinit */
    HAL_NVIC_DisableIRQ(SDMMC2_IRQn);
  }
}

int sdmmc_init() {
  HAL_SD_CardCIDTypedef pCID;
  HAL_SD_CardCSDTypedef pCSD;

  SDHandle.Instance = SDMMC2;
  HAL_SD_DeInit(&SDHandle);
  SDHandle.Init.ClockEdge = SDMMC_CLOCK_EDGE_RISING;
  SDHandle.Init.ClockPowerSave = SDMMC_CLOCK_POWER_SAVE_DISABLE;
  SDHandle.Init.BusWide = SDMMC_BUS_WIDE_4B;
  SDHandle.Init.HardwareFlowControl = SDMMC_HARDWARE_FLOW_CONTROL_ENABLE;
  SDHandle.Init.ClockDiv = 23;

  if (HAL_SD_Init(&SDHandle) != HAL_OK) {
    printf_("HAL_SD_Init failed\n");
    return -1;
  }
  // if (HAL_SD_Erase(&SDHandle, ADDRESS, ADDRESS + BUFFER_SIZE) != HAL_OK) {
  //   printf_("HAL_SD_Erase failed\n");
  //   return -1;
  // }
  if (Wait_SDCARD_Ready() != HAL_OK) {
    printf_("Wait_SDCARD_Ready failed\n");
    return -1;
  }

  HAL_SD_GetCardCID(&SDHandle, &pCID);
  HAL_SD_GetCardCSD(&SDHandle, &pCSD);

  return 0;
}

void SDMMC2_IRQHandler(void) {
  HAL_SD_IRQHandler(&SDHandle);
}

/**
  * @brief SD error callbacks
  * @param hsd: SD handle
  * @retval None
  */
void HAL_SD_ErrorCallback(SD_HandleTypeDef *hsd) {
  while (1) {
    HAL_GPIO_TogglePin(LED_GPIO_Port,LED_Pin);
    HAL_Delay(1000);
  }
}


#if !SDMMC_TEST

#define READ_REQUEST 1
#define WRITE_REQUEST 2

__IO uint8_t RxCplt = 0, TxCplt = 0;

uint8_t get_RxCplt() {
  return RxCplt;
}

void set_RxCplt(uint8_t val) {
  RxCplt = val;
}

uint8_t get_TxCplt() {
  return TxCplt;
}

void set_TxCplt(uint8_t val) {
  TxCplt = val;
}

void HAL_SD_RxCpltCallback(SD_HandleTypeDef *hsd) {
    extern void io_req_cplt_callback(uint32_t req, uint8_t *addr, uint32_t size);
    io_req_cplt_callback(READ_REQUEST, hsd->pRxBuffPtr, hsd->RxXferSize);
//   extern void wake_sdmmc_reader();
//   if (RxCplt == 2) {
//     wake_sdmmc_reader();
//   }
//   RxCplt = 1;
}

void HAL_SD_TxCpltCallback(SD_HandleTypeDef *hsd) {
    extern void io_req_cplt_callback(uint32_t req, uint8_t *addr, uint32_t size);
    io_req_cplt_callback(WRITE_REQUEST, hsd->pTxBuffPtr, hsd->TxXferSize);
//   extern void wake_sdmmc_writer();
//   if (TxCplt == 2) {
//     wake_sdmmc_writer();
//   }
//   TxCplt = 1;
}

int sdmmc_read_blocks_it(uint8_t *pData, uint32_t BlockAdd, uint32_t NumberOfBlocks) {
  if (Wait_SDCARD_Ready() != HAL_OK) {
    printf_("sdmmc_read_block_it: Wait_SDCARD_Ready failed\n");
    return -1;
  }
  if (HAL_SD_ReadBlocks_IT(&SDHandle, pData, BlockAdd, NumberOfBlocks) != HAL_OK) {
    printf_("sdmmc_read_block_it: HAL_SD_ReadBlocks_IT failed\n");
    return -1;
  }
  return 0;
}

int sdmmc_write_blocks_it(uint8_t *pData, uint32_t BlockAdd, uint32_t NumberOfBlocks) {
  if (Wait_SDCARD_Ready() != HAL_OK) {
    printf_("sdmmc_write_blocks_it: Wait_SDCARD_Ready failed\n");
    return -1;
  }
  if (HAL_SD_WriteBlocks_IT(&SDHandle, pData, BlockAdd, NumberOfBlocks) != HAL_OK) {
    printf_("sdmmc_write_blocks_it: HAL_SD_WriteBlocks_IT failed\n");
    return -1;
  }
  return 0;
}

uint64_t get_sdcard_capacity(void) {
  HAL_SD_GetCardInfo(&SDHandle, &SDCardInfo);
  return (uint64_t)SDCardInfo.LogBlockNbr * SDCardInfo.LogBlockSize;
}

#else

#define COUNTOF(__BUFFER__)        (sizeof(__BUFFER__) / sizeof(*(__BUFFER__)))

#define DATA_SIZE              ((uint32_t)0x00100000U) 
#define BUFFER_SIZE            ((uint32_t)0x00008000U)
#define NB_BUFFER              DATA_SIZE / BUFFER_SIZE
#define NB_BLOCK_BUFFER        BUFFER_SIZE / BLOCKSIZE /* Number of Block by Buffer */
#define BUFFER_WORD_SIZE       (BUFFER_SIZE>>2)        /* Buffer size in Word */
#define ADDRESS                ((uint32_t)0x00000400U) /* SD Address to write/read data */
#define DATA_PATTERN           ((uint32_t)0xB5F3A5F3U)
#define BUFFERSIZE                 (COUNTOF(aTxBuffer) - 1)

__attribute__((section (".RAM_D1")))
uint8_t aTxBuffer[BUFFER_WORD_SIZE*4];

__attribute__((section (".RAM_D1")))
uint8_t aRxBuffer[BUFFER_WORD_SIZE*4];

__IO uint8_t RxCplt, TxCplt;


/**
  * @brief Rx Transfer completed callbacks
  * @param hsd: SD handle
  * @retval None
  */
void HAL_SD_RxCpltCallback(SD_HandleTypeDef *hsd) {
  RxCplt=1;
}


/**
  * @brief Tx Transfer completed callbacks
  * @param hsd: SD handle
  * @retval None
  */
void HAL_SD_TxCpltCallback(SD_HandleTypeDef *hsd) {
  TxCplt=1;
}

int sdmmc_test() {
  __IO uint8_t step = 0;
  uint32_t start_time = 0;
  uint32_t stop_time = 0;
  uint32_t loop_index = 0;
  uint32_t index = 0;

  while (1) {
    switch (step) {
      case 0: {
        for (index = 0; index < BUFFERSIZE; index++) {
          aTxBuffer[index] = DATA_PATTERN + index;
        }
        printf(" ****************** Start Write test ******************* \n");
        printf(" - Buffer size to write: %lu MB   \n", (DATA_SIZE>>20));
        index = 0;
        start_time = HAL_GetTick();
        step++;
      } break;
      case 1: {
        TxCplt = 0;
        if(Wait_SDCARD_Ready() != HAL_OK) {
          printf_("Wait_SDCARD_Ready failed\n");
          return -1;
        }
        if(HAL_SD_WriteBlocks_IT(&SDHandle, aTxBuffer, ADDRESS + (loop_index * NB_BLOCK_BUFFER), NB_BLOCK_BUFFER) != HAL_OK) {
          printf_("HAL_SD_WriteBlocks_IT failed\n");
          return -2;
        }
        step++;
      } break;
      case 2: {
        if(TxCplt != 0) {
          index++;
          if(index < NB_BUFFER) {
            step--;
          } else {
            stop_time = HAL_GetTick();
            printf(" - Write Time(ms): %lu  -  Write Speed: %02.2f MB/s  \n", stop_time - start_time, (float)((float)(DATA_SIZE>>10)/(float)(stop_time - start_time)));
            step++;
          }
        }
      } break;
      case 3: {
        for (index = 0; index < BUFFERSIZE; index++) {
          aRxBuffer[index] = 0;
        }
        printf(" ******************* Start Read test ******************* \n");
        printf(" - Buffer size to read: %lu MB   \n", (DATA_SIZE>>20));
        start_time = HAL_GetTick();
        index = 0;
        step++;
      } break;
      case 4: {
        if(Wait_SDCARD_Ready() != HAL_OK) {
          printf_("Wait_SDCARD_Ready failed\n");
          return -3;
        }
        RxCplt = 0;
        if(HAL_SD_ReadBlocks_IT(&SDHandle, aRxBuffer, ADDRESS + (loop_index * NB_BLOCK_BUFFER), NB_BLOCK_BUFFER) != HAL_OK) {
          printf_("HAL_SD_ReadBlocks_IT failed\n");
          return -4;
        }
        step++;
      } break;
      case 5: {
        if(RxCplt != 0) {
          index++;
          if(index < NB_BUFFER) {
            step--;
          } else {
            stop_time = HAL_GetTick();
            printf(" - Read Time(ms): %lu  -  Read Speed: %02.2f MB/s  \n", stop_time - start_time, (float)((float)(DATA_SIZE>>10)/(float)(stop_time - start_time)));
            step++;
          }
        }
      } break;
      case 6: {
        index=0;
        printf(" ********************* Check data ********************** \n");
        while((index<BUFFERSIZE) && (aRxBuffer[index] == aTxBuffer[index])) {
          index++;
        }
        
        if(index != BUFFERSIZE) {
          printf(" - Check data Error !!!!   \n");
          return -5;
        }
        printf(" - Check data OK  \n");
        step = 0;
        loop_index ++;
      } break;
      default: {
        printf_("unexpected step\n");
        return -6;
      }
    }
    if (loop_index > 20) {
      break;
    }
  }

  return 0;
}

#endif